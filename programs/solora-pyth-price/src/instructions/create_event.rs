use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use chrono::{Datelike, NaiveDateTime, Timelike};
use clockwork_sdk::{
    ID as thread_program_ID,
    cpi::{
        thread_create, thread_update,
        ThreadCreate, ThreadUpdate,
    },
    state::{Trigger, Thread, ThreadSettings},
    ThreadProgram,
};
use pyth_sdk_solana::{load_price_feed_from_account_info, Price};
use solana_program::instruction::Instruction;
use crate::state::{Event, EVENT_SIZE, EVENT_VERSION, EventConfig, MAX_PRICE_DECIMALS, Outcome};
use crate::error::Error;
use crate::util::transfer;

#[derive(Accounts)]
pub struct CreateEvent<'info> {
    /// CHECK: Allow any account to be the settle authority
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0],
        has_one = authority,
        has_one = pyth_feed,
        has_one = currency_mint
    )]
    pub event_config: Box<Account<'info, EventConfig>>,

    #[account(
        init,
        seeds = [
            b"event".as_ref(),
            event_config.key().as_ref(),
            &event_config.next_event_start.to_le_bytes()
        ],
        bump,
        space = EVENT_SIZE,
        payer = authority,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Should be a valid pyth feed
    #[account()]
    pub pyth_feed: UncheckedAccount<'info>,

    /// CHECK: Allow any account to be the fee account
    #[account()]
    pub fee_account: UncheckedAccount<'info>,

    pub currency_mint: Account<'info, Mint>,

    #[account(
        mut,
        address = Thread::pubkey(event.key(), "event_lock".into())
    )]
    pub lock_thread: SystemAccount<'info>,

    #[account(
        mut,
        address = Thread::pubkey(event.key(), "event_settle".into())
    )]
    pub settle_thread: SystemAccount<'info>,

    #[account(address = thread_program_ID)]
    pub clockwork: Program<'info, ThreadProgram>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_event<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
    fee_bps: u32,
) -> Result<()> {
    let timestamp = Clock::get()?.unix_timestamp;
    let authority = &ctx.accounts.authority;
    let lock_thread = &ctx.accounts.lock_thread;
    let settle_thread = &ctx.accounts.settle_thread;
    let system_program = &ctx.accounts.system_program;
    let clockwork = &ctx.accounts.clockwork;

    let event_config = &mut ctx.accounts.event_config;
    let current_event_start = event_config.next_event_start;
    let lock_time = current_event_start + event_config.interval_seconds as i64;
    if lock_time < timestamp {
        return err!(Error::InvalidLockTime)
    }

    let wait_period = event_config.interval_seconds;
    event_config.next_event_start = lock_time;
    msg!("event start: {}, lock: {}, settle: {}", current_event_start, lock_time, event_config.next_event_start);

    let price_feed = load_price_feed_from_account_info(&ctx.accounts.pyth_feed.to_account_info()).unwrap();
    let price: Price = price_feed.get_price_unchecked();
    msg!("price.expo: {}", price.expo);
    let pyth_feed_decimals = (price.expo * -1) as u8;

    let event = &mut ctx.accounts.event;
    event.bump = [*ctx.bumps.get("event").unwrap()];
    event.version = EVENT_VERSION;
    event.event_config = event_config.key();
    event.lock_thread = ctx.accounts.lock_thread.key();
    event.settle_thread = ctx.accounts.settle_thread.key();
    event.fee_account = ctx.accounts.fee_account.key();
    event.fee_bps = fee_bps;
    event.start_time = current_event_start;
    event.lock_time = lock_time;
    event.wait_period = wait_period;
    event.outcome = Outcome::Undrawn;
    // Max 4 decimals to consider
    event.price_decimals = if pyth_feed_decimals > MAX_PRICE_DECIMALS {
        MAX_PRICE_DECIMALS
    } else {
        pyth_feed_decimals
    };

    // build set_lock_price ix
    let set_lock_price_ix = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(event_config.key(), false),
            AccountMeta::new(event.key(), false),
            AccountMeta::new_readonly(event_config.pyth_feed, false),
            AccountMeta::new(lock_thread.key(), true),
        ],
        data: clockwork_sdk::utils::anchor_sighash("set_lock_price").into(),
    };

    let start_time_bytes = &event.start_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(start_time_bytes);

    let datetime = NaiveDateTime::from_timestamp_opt(lock_time, 0)
        .ok_or(Error::InvalidLockTime)?;
    let (sec, min, hour, day, month, year) =
        (datetime.second(), datetime.minute(), datetime.hour(), datetime.day(), datetime.month(), datetime.year());
    let schedule = format!("{} {} {} {} {} * {}", sec, min, hour, day, month, year);

    // initialize thread
    thread_create(
        CpiContext::new_with_signer(
            clockwork.to_account_info(),
            ThreadCreate {
                authority: event.to_account_info(),
                payer: authority.to_account_info(),
                thread: lock_thread.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[&auth_seeds],
        ),
        "event_lock".into(),
        set_lock_price_ix.into(),
        Trigger::Cron {
            schedule: schedule.into(),
            skippable: false,
        },
    )?;

    // Higher than default fee to prioritize
    let thread_fee = 10_000u64;
    // set the rate limit of the thread to crank 1 time per slot
    thread_update(
        CpiContext::new_with_signer(
            clockwork.to_account_info(),
            ThreadUpdate {
                authority: event.to_account_info(),
                thread: lock_thread.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[&auth_seeds],
        ),
        ThreadSettings {
            fee: Some(thread_fee),
            kickoff_instruction: None,
            rate_limit: Some(1),
            trigger: None,
        },
    )?;

    // build settle_event ix
    let settle_event_ix = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(event_config.key(), false),
            AccountMeta::new(event.key(), false),
            AccountMeta::new_readonly(event_config.pyth_feed, false),
            AccountMeta::new(settle_thread.key(), true),
        ],
        data: clockwork_sdk::utils::anchor_sighash("settle_event").into(),
    };

    let start_time_bytes = &event.start_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(start_time_bytes);

    let datetime = NaiveDateTime::from_timestamp_opt(lock_time + wait_period as i64, 0)
        .ok_or(Error::InvalidLockTime)?;
    let (sec, min, hour, day, month, year) =
        (datetime.second(), datetime.minute(), datetime.hour(), datetime.day(), datetime.month(), datetime.year());
    let schedule = format!("{} {} {} {} {} * {}", sec, min, hour, day, month, year);

    // initialize thread
    thread_create(
        CpiContext::new_with_signer(
            clockwork.to_account_info(),
            ThreadCreate {
                authority: event.to_account_info(),
                payer: authority.to_account_info(),
                thread: settle_thread.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[&auth_seeds],
        ),
        "event_settle".into(),
        settle_event_ix.into(),
        Trigger::Cron {
            schedule: schedule.into(),
            skippable: false,
        },
    )?;

    // set the rate limit of the thread to crank 1 time per slot
    thread_update(
        CpiContext::new_with_signer(
            clockwork.to_account_info(),
            ThreadUpdate {
                authority: event.to_account_info(),
                thread: settle_thread.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[&auth_seeds],
        ),
        ThreadSettings {
            fee: Some(thread_fee),
            kickoff_instruction: None,
            rate_limit: Some(1),
            trigger: None,
        },
    )?;

    // Transfer the thread fees to the threads
    transfer(
        &authority.to_account_info(),
        &lock_thread.to_account_info(),
        None,
        None,
        None,
        None,
        None,
        None,
        &ctx.accounts.system_program.to_account_info(),
        None,
        None,
        None,
        thread_fee
    )?;

    transfer(
        &authority.to_account_info(),
        &settle_thread.to_account_info(),
        None,
        None,
        None,
        None,
        None,
        None,
        &ctx.accounts.system_program.to_account_info(),
        None,
        None,
        None,
        thread_fee
    )?;

    emit!(EventCreated {
        event_config: event_config.key(),
        event: event.key(),
        authority: event_config.authority,
        pyth_feed: event_config.pyth_feed,
        price_decimals: event.price_decimals,
        fee_bps,
        fee_account: event.fee_account,
        start_time: event.start_time,
        lock_time,
        wait_period,
        currency_mint: event_config.currency_mint,
    });
    Ok(())
}

#[event]
pub struct EventCreated {
    pub event_config: Pubkey,
    pub event: Pubkey,
    pub authority: Pubkey,
    pub pyth_feed: Pubkey,
    pub price_decimals: u8,
    pub fee_bps: u32,
    pub fee_account: Pubkey,
    pub start_time: i64,
    pub lock_time: i64,
    pub wait_period: u32,
    pub currency_mint: Pubkey
}