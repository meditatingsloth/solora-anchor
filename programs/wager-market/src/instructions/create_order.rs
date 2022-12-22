use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::state::{Event, Fill, Order, ORDER_SIZE};
use crate::error::Error;

#[derive(Accounts)]
pub struct CreateOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    init,
    seeds = [b"order".as_ref(), event.key().as_ref(), authority.key().as_ref()],
    bump,
    space = ORDER_SIZE,
    payer = authority,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account()]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
    outcome: u8,
    bet_amount: u64,
    ask_bps: u32,
) -> Result<()> {
    if ctx.accounts.event.is_settled {
        return err!(Error::EventSettled);
    }

    let order = &mut ctx.accounts.order;
    order.authority = ctx.accounts.authority.key();
    order.event = ctx.accounts.event.key();
    order.outcome = outcome;
    order.bet_amount = bet_amount;
    order.ask_bps = ask_bps;
    order.fills = Vec::new();



    Ok(())
}