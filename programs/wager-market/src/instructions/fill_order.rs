use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::state::{Event, Fill, FILL_SIZE, Order, ORDER_SIZE};
use crate::error::Error;

#[derive(Accounts)]
pub struct FillOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), order.authority.as_ref()],
    bump,
    realloc = Order::space(order.fills.len() as usize + 1),
    realloc::payer = authority,
    realloc::zero = false,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account()]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn fill_order<'info>(
    ctx: Context<'_, '_, '_, 'info, FillOrder<'info>>,
    outcome: u8,
    fill_amount: u64,
) -> Result<()> {
    if ctx.accounts.event.is_settled {
        return err!(Error::EventSettled);
    }

    if outcome == ctx.accounts.order.outcome {
        return err!(Error::InvalidOutcome);
    }

    let total_filled = ctx.accounts.order.fills.iter().fold(0 as u64, |acc, fill| acc + fill.fill_amount);
    msg!("total_filled: {}", total_filled);
    let total_ask = (ctx.accounts.order.bet_amount as u128)
        .checked_mul(ctx.accounts.order.ask_bps as u128)
        .ok_or(Error::CalculationOverflow)?
        .checked_div(10000 as u128)
        .ok_or(Error::CalculationOverflow)? as u64;
    msg!("total_ask: {}", total_ask);
    let remaining_ask = total_ask.checked_sub(total_filled)
        .ok_or(Error::CalculationOverflow)?;
    msg!("remaining_ask: {}", remaining_ask);

    if fill_amount > remaining_ask {
        return err!(Error::FillAmountTooLarge);
    }

    // TODO: Send fill amount to the Order

    let order = &mut ctx.accounts.order;
    order.fills.push(Fill {
        authority: ctx.accounts.authority.key(),
        outcome,
        fill_amount,
    });

    Ok(())
}