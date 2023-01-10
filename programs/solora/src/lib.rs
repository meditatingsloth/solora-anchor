use anchor_lang::prelude::*;
use instructions::*;

mod state;
mod error;
mod instructions;
mod util;

declare_id!("8b5j5Ua8jBDqnCZNB22NJAedd5TBs5NBAjqF65q8BpuS");

#[program]
pub mod solora {
    use super::*;

    pub fn create_event<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
        id: [u8; 32],
        fee_account: Pubkey,
        fee_bps: u32,
        metadata_uri: String,
    ) -> Result<()> {
        instructions::create_event(ctx, id, fee_account, fee_bps, metadata_uri)
    }

    pub fn create_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
        outcome: u8,
        amount: u64,
        ask_bps: u32,
        expiry: i64
    ) -> Result<()> {
        instructions::create_order(ctx, outcome, amount, ask_bps, expiry)
    }

    pub fn cancel_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelOrder<'info>>,
        index: u32,
        amount: u64
    ) -> Result<()> {
        instructions::cancel_order(ctx, index, amount)
    }

    pub fn fill_order<'info>(
        ctx: Context<'_, '_, '_, 'info, FillOrder<'info>>,
        index: u32,
        outcome: u8,
        amount: u64
    ) -> Result<()> {
        instructions::fill_order(ctx, index, outcome, amount)
    }

    pub fn settle_event<'info>(
        ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
        id: [u8; 32],
        outcome: u8
    ) -> Result<()> {
        instructions::settle_event(ctx, id, outcome)
    }

    pub fn settle_fill<'info>(
        ctx: Context<'_, '_, '_, 'info, SettleFill<'info>>,
        order_index: u32,
        fill_index: u32
    ) -> Result<()> {
        instructions::settle_fill(ctx, order_index, fill_index)
    }
}
