use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use crate::state::{EventConfig, EVENT_CONFIG_SIZE, EVENT_CONFIG_VERSION};

#[derive(Accounts)]
pub struct CreateEventConfig<'info> {
    /// CHECK: Allow any account to be the settle authority
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"event_config".as_ref(),
            authority.key().as_ref(),
            pyth_feed.key().as_ref(),
            currency_mint.key().as_ref()
        ],
        bump,
        space = EVENT_CONFIG_SIZE,
        payer = authority
    )]
    pub event_config: Box<Account<'info, EventConfig>>,

    /// CHECK: Should be a valid pyth feed
    #[account()]
    pub pyth_feed: UncheckedAccount<'info>,

    pub currency_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_event_config<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateEventConfig<'info>>,
    interval_seconds: u32,
    next_event_start: i64
) -> Result<()> {
    let event_config = &mut ctx.accounts.event_config;
    event_config.bump = [*ctx.bumps.get("event_config").unwrap()];
    event_config.version = EVENT_CONFIG_VERSION;
    event_config.authority = ctx.accounts.authority.key();
    event_config.pyth_feed = ctx.accounts.pyth_feed.key();
    event_config.currency_mint = ctx.accounts.currency_mint.key();
    event_config.interval_seconds = interval_seconds;
    event_config.next_event_start = next_event_start;

    Ok(())
}