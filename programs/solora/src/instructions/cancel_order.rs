use anchor_lang::prelude::*;
use crate::state::{Event, Fill, Order, ORDER_SIZE};
use crate::error::Error;
use crate::util::{assert_is_ata, is_default, transfer, transfer_sol};

#[derive(Accounts)]
#[instruction(index: u32)]
pub struct CancelOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &index.to_le_bytes()],
    bump,
    has_one = authority,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = !event.is_settled @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
}

pub fn cancel_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CancelOrder<'info>>,
    index: u32,
) -> Result<()> {
    // TODO: Fail if no remaining bet amount to be filled

    // TODO: Close if no fills

    // TODO: Reduce bet amount if partially filled

    Ok(())
}