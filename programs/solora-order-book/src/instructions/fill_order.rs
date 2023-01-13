use anchor_lang::prelude::*;
use crate::state::{Event, Fill, Order};
use crate::error::Error;
use crate::util::{is_native_mint, transfer, transfer_sol};

#[derive(Accounts)]
#[instruction(index: u32, outcome: u8)]
pub struct FillOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &index.to_le_bytes()],
    bump = order.bump[0],
    realloc = Order::space(order.fills.len() as usize + 1),
    realloc::payer = authority,
    realloc::zero = false,
    constraint = order.outcome != outcome @ Error::InvalidOutcome,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = event.outcome == 0 @ Error::EventSettled,
    constraint = outcome != event.outcome @ Error::InvalidOutcome,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn fill_order<'info>(
    ctx: Context<'_, '_, '_, 'info, FillOrder<'info>>,
    _index: u32,
    outcome: u8,
    amount: u64,
) -> Result<()> {
    let timestamp = Clock::get()?.unix_timestamp;

    if ctx.accounts.order.expiry != 0 {
        if ctx.accounts.order.expiry <= timestamp {
            return err!(Error::OrderExpired);
        }
    }

    if ctx.accounts.event.close_time != 0 &&
        timestamp >= ctx.accounts.event.close_time {
        return err!(Error::EventClosed);
    }

    if amount > ctx.accounts.order.remaining_ask {
        return err!(Error::FillAmountTooLarge);
    }

    let order_obligation = (amount as u128)
        .checked_mul(10000 as u128).unwrap()
        .checked_div(ctx.accounts.order.ask_bps as u128).unwrap() as u64;
    // Ensure the obligation is not too large.
    let safe_amount = (order_obligation as u128)
        .checked_mul(ctx.accounts.order.ask_bps as u128).unwrap()
        .checked_div(10000 as u128).unwrap() as u64;

    if is_native_mint(ctx.accounts.order.currency_mint) {
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.order.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            safe_amount,
        )?;
    } else {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        let order_currency_account = next_account_info(remaining_accounts)?;
        let user_currency_account = next_account_info(remaining_accounts)?;
        let token_program = next_account_info(remaining_accounts)?;

        transfer(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.order.to_account_info(),
            user_currency_account.into(),
            order_currency_account.into(),
            currency_mint.into(),
            None,
            None,
            token_program.into(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            None,
            None,
            safe_amount,
        )?;
    }

    let fill_index = ctx.accounts.order.fills.len() as u32;
    let order = &mut ctx.accounts.order;
    order.remaining_ask = order.remaining_ask.checked_sub(safe_amount)
        .ok_or(Error::OverflowError)?;
    order.fills.push(Fill {
        index: fill_index,
        authority: ctx.accounts.authority.key(),
        outcome,
        amount: safe_amount,
        is_settled: false,
    });

    Ok(())
}