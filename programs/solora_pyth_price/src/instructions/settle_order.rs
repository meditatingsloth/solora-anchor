use crate::error::Error;
use crate::state::{Event, Order, Outcome};
use crate::util::{is_native_mint, transfer};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SettleOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Safe due to event constraint
    #[account(mut)]
    pub fee_account: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"order".as_ref(), event.key().as_ref(), authority.key().as_ref()],
        bump = order.bump[0],
        has_one = authority
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
        mut,
        constraint = event.outcome != Outcome::Undrawn @ Error::EventNotSettled,
        constraint = order.outcome == event.outcome @ Error::InvalidOutcome,
        has_one = fee_account
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn settle_order<'info>(ctx: Context<'_, '_, '_, 'info, SettleOrder<'info>>) -> Result<()> {
    let event = &ctx.accounts.event;
    let timestamp = Clock::get()?.unix_timestamp;

    if ctx.accounts.event.close_time != 0 && timestamp >= ctx.accounts.event.close_time {
        return err!(Error::EventClosed);
    }

    let (winning_pool, losing_pool): (u128, u128);

    if event.outcome == Outcome::Up {
        winning_pool = event.up_amount;
        losing_pool = event.down_amount;
    } else {
        losing_pool = event.up_amount;
        winning_pool = event.down_amount;
    }

    // Divide the losing pool by winning for multiplier
    let mut amount = u64::try_from(
        (ctx.accounts.order.amount as u128)
            .checked_mul(losing_pool)
            .ok_or(Error::OverflowError)?
            .checked_div(winning_pool)
            .ok_or(Error::OverflowError)?,
    )
    .map_err(|_| Error::OverflowError)?;

    let is_native = is_native_mint(event.currency_mint);
    let seeds = ctx.accounts.order.auth_seeds();

    let fee = amount
        .checked_mul(ctx.accounts.event.fee_bps as u64)
        .ok_or(Error::OverflowError)?
        .checked_div(10000)
        .ok_or(Error::OverflowError)?;

    let remaining_accounts = &mut ctx.remaining_accounts.iter();
    let (
        currency_mint,
        order_currency_account,
        user_currency_account,
        fee_currency_account,
        token_program,
        ata_program,
        rent,
    ) = if is_native {
        (
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
        )
    } else {
        (None, None, None, None, None, None, None)
    };

    if fee > 0 {
        amount = amount.checked_sub(fee).unwrap();
    }

    transfer(
        &ctx.accounts.order.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        order_currency_account,
        user_currency_account,
        currency_mint,
        Option::from(&ctx.accounts.authority.to_account_info()),
        ata_program,
        token_program,
        &ctx.accounts.system_program.to_account_info(),
        rent,
        Some(&seeds),
        None,
        amount,
    )?;

    if fee > 0 {
        transfer(
            &ctx.accounts.order.to_account_info(),
            &ctx.accounts.fee_account.to_account_info(),
            order_currency_account,
            fee_currency_account,
            currency_mint,
            Option::from(&ctx.accounts.authority.to_account_info()),
            ata_program,
            token_program,
            &ctx.accounts.system_program.to_account_info(),
            rent,
            Some(&seeds),
            None,
            fee,
        )?;
    }

    Ok(())
}
