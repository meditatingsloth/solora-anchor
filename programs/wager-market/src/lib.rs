use anchor_lang::prelude::*;
use instructions::*;

mod state;
mod error;
mod instructions;
mod util;

use crate::error::Error;

declare_id!("8b5j5Ua8jBDqnCZNB22NJAedd5TBs5NBAjqF65q8BpuS");

#[program]
pub mod wager_market {
    use super::*;

    pub fn create_event<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
        id: [u8; 32],
        metadata_uri: String,
    ) -> Result<()> {
        instructions::create_event(ctx, id, metadata_uri)
    }

    pub fn create_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
        outcome: u8,
        bet_amount: u64,
        ask_bps: u32,
    ) -> Result<()> {
        instructions::create_order(ctx, outcome, bet_amount, ask_bps)
    }

    pub fn fill_order<'info>(
        ctx: Context<'_, '_, '_, 'info, FillOrder<'info>>,
        outcome: u8,
        fill_amount: u64
    ) -> Result<()> {
        instructions::fill_order(ctx, outcome, fill_amount)
    }

}
