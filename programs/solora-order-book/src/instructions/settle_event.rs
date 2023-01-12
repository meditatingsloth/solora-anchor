use anchor_lang::prelude::*;
use crate::state::{Event};
use crate::error::Error;

#[derive(Accounts)]
#[instruction(id: [u8; 32], outcome: u8)]
pub struct SettleEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"event".as_ref(), id.as_ref()],
    bump = event.bump[0],
    has_one = authority,
    constraint = event.outcome == 0 @ Error::EventSettled,
    constraint = outcome != 0 @ Error::InvalidOutcome,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
}

pub fn settle_event<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
    _id: [u8; 32],
    outcome: u8,
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    event.outcome = outcome;

    Ok(())
}