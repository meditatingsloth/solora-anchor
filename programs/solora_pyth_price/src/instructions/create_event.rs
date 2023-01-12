use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use crate::state::{Event, EVENT_SIZE, Outcome};
use crate::error::Error;

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

    let event = &mut ctx.accounts.event;
    event.bump = [*ctx.bumps.get("event").unwrap()];
    event.authority = ctx.accounts.authority.key();
    event.pyth_feed = ctx.accounts.pyth_feed.key();
    event.fee_account = ctx.accounts.fee_account.key();
    event.fee_bps = fee_bps;
    event.lock_time = lock_time;
    event.wait_period = wait_period;
    event.currency_mint = ctx.accounts.currency_mint.key();
    event.outcome = Outcome::Undrawn;

    Ok(())
}