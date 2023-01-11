use anchor_lang::prelude::*;
use crate::state::{Event, Outcome};
use crate::error::Error;

#[derive(Accounts)]
#[instruction(id: [u8; 32], outcome: Outcome)]
pub struct SettleEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"event".as_ref(), id.as_ref()],
        bump = event.bump[0],
        has_one = authority,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
        constraint = outcome != Outcome::Undrawn @ Error::InvalidOutcome,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
}

pub fn settle_event<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
    _id: [u8; 32],
    outcome: Outcome,
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    event.outcome = outcome;

    Ok(())
}