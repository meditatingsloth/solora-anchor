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
use solana_program::instruction::Instruction;
use crate::state::{Event, EVENT_SIZE, Outcome};
use crate::error::Error;
use crate::util::transfer;

#[derive(Accounts)]
#[instruction(lock_time: i64)]
pub struct CreateEvent<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Allow any account to be the settle authority
    #[account()]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            b"event".as_ref(),
            pyth_feed.key().as_ref(),
            fee_account.key().as_ref(),
            currency_mint.key().as_ref(),
            &lock_time.to_le_bytes()
        ],
        bump,
        space = EVENT_SIZE,
        payer = payer,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: TODO: Does pyth do their own validation when reading price?
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
    lock_time: i64,
    wait_period: u32,
    fee_bps: u32,
) -> Result<()> {
    if lock_time <= Clock::get()?.unix_timestamp {
        return err!(Error::InvalidLockTime);
    }

    let clockwork = &ctx.accounts.clockwork;
    let lock_thread = &ctx.accounts.lock_thread;
    let settle_thread = &ctx.accounts.settle_thread;
    let payer = &ctx.accounts.payer;
    let system_program = &ctx.accounts.system_program;

    let event = &mut ctx.accounts.event;
    event.bump = [*ctx.bumps.get("event").unwrap()];
    event.authority = ctx.accounts.authority.key();
    event.lock_thread = ctx.accounts.lock_thread.key();
    event.settle_thread = ctx.accounts.settle_thread.key();
    event.pyth_feed = ctx.accounts.pyth_feed.key();
    event.fee_account = ctx.accounts.fee_account.key();
    event.fee_bps = fee_bps;
    event.lock_time = lock_time;
    event.wait_period = wait_period;
    event.currency_mint = ctx.accounts.currency_mint.key();
    event.outcome = Outcome::Undrawn;

    // build set_lock_price ix
    let set_lock_price_ix = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(event.key(), false),
            AccountMeta::new_readonly(event.pyth_feed, false),
            AccountMeta::new(lock_thread.key(), true),
        ],
        data: clockwork_sdk::utils::anchor_sighash("set_lock_price").into(),
    };

    let lock_time_bytes = &event.lock_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(lock_time_bytes);

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
                payer: payer.to_account_info(),
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
            AccountMeta::new(event.key(), false),
            AccountMeta::new_readonly(event.pyth_feed, false),
            AccountMeta::new(settle_thread.key(), true),
        ],
        data: clockwork_sdk::utils::anchor_sighash("settle_event").into(),
    };

    let lock_time_bytes = &event.lock_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(lock_time_bytes);

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
                payer: payer.to_account_info(),
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
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.lock_thread.to_account_info(),
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
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.settle_thread.to_account_info(),
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

    Ok(())
}