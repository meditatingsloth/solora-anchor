use anchor_lang::prelude::*;
use instructions::*;
use state::Outcome;

mod state;
mod error;
mod instructions;
mod util;

declare_id!("14SStXZMvqahGWKeU3699C6of1dcmvvaa3b5ESsvz6U2");

#[program]
pub mod solora_pyth_price {
    use crate::state::Outcome;

    use super::*;

    pub fn create_event<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
        id: [u8; 32],
        fee_account: Pubkey,
        fee_bps: u32,
        end_time: i64,
        metadata_uri: String,
    ) -> Result<()> {
        instructions::create_event(ctx, id, fee_account, fee_bps, end_time, metadata_uri)
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
        id: [u8; 32],
        outcome: Outcome
    ) -> Result<()> {
        instructions::settle_event(ctx, id, outcome)
    }
}
