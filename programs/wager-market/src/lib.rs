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

}
