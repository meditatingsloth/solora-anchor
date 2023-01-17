use anchor_lang::prelude::*;
use crate::state::{EventConfig};

#[derive(Accounts)]
pub struct UpdateEventConfig<'info> {
    /// CHECK: Allow any account to be the settle authority
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"event_config".as_ref(),
            authority.key().as_ref(),
            event_config.pyth_feed.key().as_ref(),
            event_config.currency_mint.key().as_ref()
        ],
        bump = event_config.bump[0],
        has_one = authority
    )]
    pub event_config: Box<Account<'info, EventConfig>>,
}

pub fn update_event_config<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateEventConfig<'info>>,
    interval_seconds: u32,
    next_event_start: i64
) -> Result<()> {
    let event_config = &mut ctx.accounts.event_config;
    event_config.interval_seconds = interval_seconds;
    event_config.next_event_start = next_event_start;

    Ok(())
}