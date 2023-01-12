use anchor_lang::prelude::*;
use instructions::*;
use state::Outcome;

mod state;
mod error;
mod instructions;
mod util;

declare_id!("SPPQ71aSCVntCUUWYxpkQa6awxpfgWsuU13H4WTwEAG");

#[program]
pub mod solora_pyth_price {
    use super::*;

    pub fn create_event<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
        lock_time: i64,
        wait_period: u32,
        fee_bps: u32,
    ) -> Result<()> {
        instructions::create_event(ctx, lock_time, wait_period, fee_bps)
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
}
