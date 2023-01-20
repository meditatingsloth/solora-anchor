use anchor_lang::prelude::*;
use crate::state::{Event, EventConfig, Outcome};
use crate::error::Error;

#[derive(Accounts)]
pub struct SettleExpiredEvent<'info> {
    /// CHECK: Safe due to event config constraint
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0],
        has_one = authority,
    )]
    pub event_config: Box<Account<'info, EventConfig>>,

    #[account(
        mut,
        seeds = [
            b"event".as_ref(),
            event_config.key().as_ref(),
            &event.start_time.to_le_bytes()
        ],
        bump = event.bump[0],
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>
}

pub fn settle_expired_event<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleExpiredEvent<'info>>,
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    event.outcome = Outcome::Invalid;

    let timestamp = Clock::get()?.unix_timestamp;
    // Event is expired if the current time is a waiting period after the end of the event
    if timestamp < event.lock_time + (event.wait_period as i64).checked_mul(2).unwrap() {
        return err!(Error::EventNotExpired);
    }

    emit!(EventSettled {
        event_config: event.event_config,
        event: event.key(),
        settle_price: 0,
        outcome: event.outcome
    });

    Ok(())
}

#[event]
pub struct EventSettled {
    pub event_config: Pubkey,
    pub event: Pubkey,
    pub settle_price: u64,
    pub outcome: Outcome
}