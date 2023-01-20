use crate::error::Error;
use crate::state::{Event, EventConfig, Order, Outcome};
use crate::util::{is_native_mint, transfer, transfer_sol_pda};
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
        has_one = fee_account,
        has_one = event_config
    )]
    pub event: Box<Account<'info, Event>>,

    #[account(
        mut,
        close = authority,
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
    let event_config = &ctx.accounts.event_config;
    let is_native = is_native_mint(event_config.currency_mint);
    let event = &ctx.accounts.event;
    let order = &ctx.accounts.order;
    let both_sides_entered = event.up_amount > 0 && event.down_amount > 0;

    let earned_amount = if both_sides_entered &&
        (event.outcome == Outcome::Up || event.outcome == Outcome::Down) {
        let (winning_pool, losing_pool): (u128, u128);

        if event.outcome == Outcome::Up {
            winning_pool = event.up_amount;
            losing_pool = event.down_amount;
        } else {
            winning_pool = event.down_amount;
            losing_pool = event.up_amount;
        }

        // Divide the losing pool by winning for earnings multiplier
        (order.amount as u128)
            .checked_mul(losing_pool)
            .ok_or(Error::OverflowError)?
            .checked_div(winning_pool)
            .ok_or(Error::OverflowError)? as u64
    } else {
        // Nothing earned if only one-sided bets or outcome wasn't up or down
        0
    };

    msg!("earned_amount: {}", earned_amount);

    // Only take fees on earned amounts
    let fee = if earned_amount > 0 {
        (earned_amount as u128)
            .checked_mul(event.fee_bps as u128)
            .ok_or(Error::OverflowError)?
            .checked_div(10000)
            .ok_or(Error::OverflowError)? as u64
    } else {
        0
    };

    msg!("fee: {}", fee);

    let user_won = event.outcome == order.outcome;
    let amount_to_user = if !both_sides_entered {
        // User gets their original amount back if only one side had bets
        order.amount
    } else if user_won {
        // User gets their original amount back plus their earnings if they won
        earned_amount
            .checked_sub(fee)
            .ok_or(Error::OverflowError)?
            .checked_add(order.amount)
            .ok_or(Error::OverflowError)?
    } else if event.outcome != Outcome::Up &&
        event.outcome != Outcome::Down {
        // User gets their original amount back if event was invalid or cancelled
        order.amount
    } else {
        // User lost, does not get anything back
        0
    };

    msg!("amount_to_user: {}", amount_to_user);

    if amount_to_user > 0 {
        if is_native {
            transfer_sol_pda(
                &mut ctx.accounts.event.to_account_info(),
                &mut ctx.accounts.authority.to_account_info(),
                amount_to_user
            )?;

            if fee > 0 {
                transfer_sol_pda(
                    &mut ctx.accounts.event.to_account_info(),
                    &mut ctx.accounts.fee_account.to_account_info(),
                    fee
                )?;
            }
        } else {
            let remaining_accounts = &mut ctx.remaining_accounts.iter();
            let currency_mint = next_account_info(remaining_accounts)?;
            let event_currency_account = next_account_info(remaining_accounts)?;
            let user_currency_account = next_account_info(remaining_accounts)?;
            let fee_currency_account = next_account_info(remaining_accounts)?;
            let token_program = next_account_info(remaining_accounts)?;
            let ata_program = next_account_info(remaining_accounts)?;
            let rent = next_account_info(remaining_accounts)?;

            let start_time_bytes = &event.start_time.to_le_bytes();
            let auth_seeds = event.auth_seeds(start_time_bytes);

            transfer(
                &ctx.accounts.event.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                event_currency_account.into(),
                user_currency_account.into(),
                currency_mint.into(),
                Option::from(&ctx.accounts.authority.to_account_info()),
                ata_program.into(),
                token_program.into(),
                &ctx.accounts.system_program.to_account_info(),
                rent.into(),
                Some(&auth_seeds),
                None,
                amount_to_user
            )?;

            if fee > 0 {
                transfer(
                    &ctx.accounts.event.to_account_info(),
                    &ctx.accounts.fee_account.to_account_info(),
                    event_currency_account.into(),
                    fee_currency_account.into(),
                    currency_mint.into(),
                    Option::from(&ctx.accounts.authority.to_account_info()),
                    ata_program.into(),
                    token_program.into(),
                    &ctx.accounts.system_program.to_account_info(),
                    rent.into(),
                    Some(&auth_seeds),
                    None,
                    fee,
                )?;
            }
        }
    }

    let event = &mut ctx.accounts.event;
    event.orders_settled += 1;

    Ok(())
}
