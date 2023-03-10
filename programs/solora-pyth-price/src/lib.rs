use anchor_lang::prelude::*;
use instructions::*;
use state::Outcome;

pub mod state;
pub mod error;
pub mod instructions;
mod util;

declare_id!("SPPq79wtPSBeFvYJbSxS9Pj1JdbQARDWxwJBXyTVcRg");

#[program]
pub mod solora_pyth_price {
    use super::*;

    pub fn create_event_config<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEventConfig<'info>>,
        interval_seconds: u32,
        next_event_start: i64
    ) -> Result<()> {
        instructions::create_event_config(ctx, interval_seconds, next_event_start)
    }

    pub fn update_event_config<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateEventConfig<'info>>,
        interval_seconds: u32,
        next_event_start: i64
    ) -> Result<()> {
        instructions::update_event_config(ctx, interval_seconds, next_event_start)
    }

    pub fn create_event<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
        fee_bps: u32,
        initial_liquidity: u64,
        fee_burn_bps: u32,
    ) -> Result<()> {
        instructions::create_event(ctx, fee_bps, initial_liquidity, fee_burn_bps)
    }

    pub fn set_lock_price<'info>(
        ctx: Context<'_, '_, '_, 'info, SetLockPrice<'info>>,
    ) -> Result<()> {
        instructions::set_lock_price(ctx)
    }

    pub fn create_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
        outcome: Outcome,
        amount: u64,
    ) -> Result<()> {
        instructions::create_order(ctx, outcome, amount)
    }

    pub fn settle_order<'info>(
        ctx: Context<'_, '_, '_, 'info, SettleOrder<'info>>,
    ) -> Result<()> {
        instructions::settle_order(ctx)
    }

    pub fn settle_event<'info>(
        ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
    ) -> Result<()> {
        instructions::settle_event(ctx)
    }

    pub fn settle_expired_event<'info>(
        ctx: Context<'_, '_, '_, 'info, SettleExpiredEvent<'info>>,
    ) -> Result<()> {
        instructions::settle_expired_event(ctx)
    }

    pub fn close_accounts<'info>(
        ctx: Context<'_, '_, '_, 'info, CloseAccounts<'info>>,
    ) -> Result<()> {
        instructions::close_accounts(ctx)
    }
}
