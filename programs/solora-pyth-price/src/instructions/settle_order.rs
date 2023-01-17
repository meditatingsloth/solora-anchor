use crate::error::Error;
use crate::state::{Event, EventConfig, Order, Outcome};
use crate::util::{is_native_mint, transfer};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SettleOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0]
    )]
    pub event_config: Box<Account<'info, EventConfig>>,

    #[account(
        mut,
        seeds = [
            b"event".as_ref(),
            event_config.key().as_ref(),
            &event.start_time.to_le_bytes()
        ],
        bump = event.bump[0],
        constraint = event.outcome != Outcome::Undrawn @ Error::EventNotSettled,
        constraint = event.outcome == Outcome::Invalid || order.outcome == event.outcome @ Error::InvalidOutcome,
        has_one = fee_account,
        has_one = event_config
    )]
    pub event: Box<Account<'info, Event>>,

    #[account(
        mut,
        seeds = [b"order".as_ref(), event.key().as_ref(), authority.key().as_ref()],
        bump = order.bump[0],
        has_one = event,
        has_one = authority
    )]
    pub order: Box<Account<'info, Order>>,

    /// CHECK: Safe due to event constraint
    #[account(mut)]
    pub fee_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn settle_order<'info>(ctx: Context<'_, '_, '_, 'info, SettleOrder<'info>>) -> Result<()> {
    let event = &ctx.accounts.event;
    let event_config = &ctx.accounts.event_config;

    let (winning_pool, losing_pool): (u128, u128);

    if event.outcome == Outcome::Up {
        winning_pool = event.up_amount;
        losing_pool = event.down_amount;
    } else if event.outcome == Outcome::Down {
        winning_pool = event.down_amount;
        losing_pool = event.up_amount;
    } else {
        winning_pool = event.up_amount + event.down_amount;
        losing_pool = 0;
    }

    // Nothing earned if invalid outcome
    let mut earned_amount = if event.outcome == Outcome::Invalid {
        0
    } else {
        // Divide the losing pool by winning for multiplier
        (ctx.accounts.order.amount as u128)
            .checked_mul(losing_pool)
            .ok_or(Error::OverflowError)?
            .checked_div(winning_pool)
            .ok_or(Error::OverflowError)? as u64
    };

    let is_native = is_native_mint(event_config.currency_mint);
    let start_time_bytes = &event.start_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(start_time_bytes);

    let fee = if event.outcome == Outcome::Invalid {
        0
    } else {
        earned_amount.checked_mul(ctx.accounts.event.fee_bps as u64)
            .ok_or(Error::OverflowError)?
            .checked_div(10000)
            .ok_or(Error::OverflowError)?
    };

    if fee > 0 {
        earned_amount = earned_amount.checked_sub(fee).unwrap();
    }

    let remaining_accounts = &mut ctx.remaining_accounts.iter();
    let (
        currency_mint,
        event_currency_account,
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

    // Transfer the earned amount + their original amount
    transfer(
        &ctx.accounts.event.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        event_currency_account,
        user_currency_account,
        currency_mint,
        Option::from(&ctx.accounts.authority.to_account_info()),
        ata_program,
        token_program,
        &ctx.accounts.system_program.to_account_info(),
        rent,
        Some(&auth_seeds),
        None,
        earned_amount.checked_add(ctx.accounts.order.amount).unwrap(),
    )?;

    if fee > 0 {
        transfer(
            &ctx.accounts.event.to_account_info(),
            &ctx.accounts.fee_account.to_account_info(),
            event_currency_account,
            fee_currency_account,
            currency_mint,
            Option::from(&ctx.accounts.authority.to_account_info()),
            ata_program,
            token_program,
            &ctx.accounts.system_program.to_account_info(),
            rent,
            Some(&auth_seeds),
            None,
            fee,
        )?;
    }

    Ok(())
}
