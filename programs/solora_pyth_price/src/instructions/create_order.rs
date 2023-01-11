use anchor_lang::prelude::*;
use crate::state::{Event, Order, ORDER_SIZE, Outcome};
use crate::error::Error;
use crate::util::{transfer, transfer_sol, is_native_mint};

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

    #[account(
        mut,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
    outcome: Outcome,
    amount: u64
) -> Result<()> {
    let timestamp = Clock::get()?.unix_timestamp;
    if ctx.accounts.event.close_time != 0 &&
        timestamp >= ctx.accounts.event.close_time {
        return err!(Error::EventClosed);
    }

    if outcome == Outcome::Undrawn {
        return err!(Error::InvalidOutcome);
    }

    let order = &mut ctx.accounts.order;
    let event = &mut ctx.accounts.event;
    order.bump = [*ctx.bumps.get("order").unwrap()];
    order.authority = ctx.accounts.authority.key();
    order.event = event.key();
    order.outcome = outcome;
    order.amount = amount;

    if outcome == Outcome::Up {
        event.up_amount += event.up_amount.checked_add(amount as u128).ok_or(Error::OverflowError)?;
        event.up_count += 1;
    }
    else {
        event.down_amount += event.down_amount.checked_add(amount as u128).ok_or(Error::OverflowError)?;
        event.down_count += 1;
    }

    // If there are remaining_accounts populated we're using an alt currency mint
    if ctx.remaining_accounts.len() == 0 {
        if !is_native_mint(ctx.accounts.event.currency_mint) {
            return err!(Error::InvalidMint);
        }
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &order.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            amount,
        )?;

    } else {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        let order_currency_account = next_account_info(remaining_accounts)?;
        let user_currency_account = next_account_info(remaining_accounts)?;
        let token_program = next_account_info(remaining_accounts)?;
        let ata_program = next_account_info(remaining_accounts)?;
        let rent = next_account_info(remaining_accounts)?;

        if ctx.accounts.event.currency_mint != currency_mint.key() {
            return err!(Error::InvalidMint);
        }

        transfer(
            &ctx.accounts.authority.to_account_info(),
            &order.to_account_info(),
            user_currency_account.into(),
            order_currency_account.into(),
            currency_mint.into(),
            Option::from(&ctx.accounts.authority.to_account_info()),
            ata_program.into(),
            token_program.into(),
            &ctx.accounts.system_program.to_account_info(),
            rent.into(),
            None,
            None,
            amount,
        )?;

    }

    Ok(())
}