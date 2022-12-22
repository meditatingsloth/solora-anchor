use anchor_lang::prelude::*;
use crate::state::{Event, Fill, Order};
use crate::error::Error;
use crate::util::{is_default, transfer, transfer_sol};

#[derive(Accounts)]
#[instruction(index: u32, outcome: u8)]
pub struct FillOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &index.to_le_bytes()],
    bump,
    realloc = Order::space(order.fills.len() as usize + 1),
    realloc::payer = authority,
    realloc::zero = false,
    constraint = order.outcome != outcome @ Error::InvalidOutcome,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = !event.is_settled @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn fill_order<'info>(
    ctx: Context<'_, '_, '_, 'info, FillOrder<'info>>,
    index: u32,
    outcome: u8,
    fill_amount: u64,
) -> Result<()> {
    if ctx.accounts.order.get_fill_index(ctx.accounts.authority.key()) != None {
        msg!("Fill already exists for {}, use update_fill instead", ctx.accounts.authority.key());
        return err!(Error::UserAlreadyFilled);
    }

    if ctx.accounts.order.expiry != -1 {
        let timestamp = Clock::get()?.unix_timestamp;
        if ctx.accounts.order.expiry <= timestamp {
            return err!(Error::OrderExpired);
        }
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

    if is_default(ctx.accounts.event.currency_mint) {
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.event.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            fill_amount,
        )?;
    } else {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        let escrow_account = next_account_info(remaining_accounts)?;
        let user_currency_account = next_account_info(remaining_accounts)?;
        let token_program = next_account_info(remaining_accounts)?;

        transfer(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.event.to_account_info(),
            user_currency_account.into(),
            escrow_account.into(),
            currency_mint.into(),
            None,
            None,
            token_program.into(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            None,
            None,
            fill_amount,
        )?;
    }

    let order = &mut ctx.accounts.order;
    order.fills.push(Fill {
        authority: ctx.accounts.authority.key(),
        outcome,
        fill_amount,
    });

    Ok(())
}