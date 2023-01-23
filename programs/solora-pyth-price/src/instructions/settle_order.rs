use crate::error::Error;
use crate::state::{Event, EventConfig, Order, Outcome};
use crate::util::{is_native_mint, transfer, transfer_sol_pda};
use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Burn;

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

    let earned_amount = get_earned_amount(
        event.up_amount,
        event.down_amount,
        event.outcome,
        order.amount,
        order.outcome
    )?;
    msg!("earned_amount: {}", earned_amount);

    // Only take fees on earned amounts
    let mut fee = get_bps_amount(earned_amount, event.fee_bps)?;
    msg!("fee: {}", fee);

    let amount_to_user = get_amount_to_user(
        event.outcome,
        event.up_amount,
        event.down_amount,
        order.outcome,
        order.amount,
        earned_amount,
        fee
    )?;
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
                Option::from(&ctx.accounts.rent.to_account_info()),
                Some(&auth_seeds),
                None,
                amount_to_user
            )?;

            if fee > 0 {
                // Burn fees if needed
                if event.fee_burn_bps > 0 {
                    let fee_burn_amount = get_bps_amount(fee, event.fee_burn_bps)?;
                    if fee_burn_amount > 0 {
                        let burn_cpi = CpiContext::new(
                            token_program.to_account_info(),
                            Burn {
                                mint: currency_mint.to_account_info(),
                                authority: ctx.accounts.event.to_account_info(),
                                from: event_currency_account.to_account_info()
                            }
                        );

                        token::burn(
                            burn_cpi,
                            fee_burn_amount
                        )?;

                        fee = fee.checked_sub(fee_burn_amount).unwrap();
                    }
                }

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
                    Option::from(&ctx.accounts.rent.to_account_info()),
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

fn get_earned_amount(
    up_amount: u128,
    down_amount: u128,
    event_outcome: Outcome,
    order_amount: u64,
    order_outcome: Outcome
) -> Result<u64> {
    if event_outcome != order_outcome {
        return Ok(0)
    }

    // Nothing earned if only one side was entered
    if up_amount == 0 || down_amount == 0 {
        return Ok(0)
    }

    let (winning_pool, losing_pool): (u128, u128);

    if event_outcome == Outcome::Up {
        winning_pool = up_amount;
        losing_pool = down_amount;
    } else if event_outcome == Outcome::Down {
        winning_pool = down_amount;
        losing_pool = up_amount;
    } else {
        return Ok(0)
    }

    // Divide the losing pool by winning for earnings multiplier
    Ok((order_amount as u128)
        .checked_mul(losing_pool)
        .ok_or(Error::OverflowError)?
        .checked_div(winning_pool)
        .ok_or(Error::OverflowError)? as u64)
}

fn get_bps_amount(
    amount: u64,
    bps: u32
) -> Result<u64> {
    if amount == 0 {
        return Ok(0)
    }

    Ok((amount as u128)
        .checked_mul(bps as u128)
        .ok_or(Error::OverflowError)?
        .checked_div(10000)
        .ok_or(Error::OverflowError)? as u64)
}

fn get_amount_to_user(
    event_outcome: Outcome,
    up_amount: u128,
    down_amount: u128,
    order_outcome: Outcome,
    order_amount: u64,
    earned_amount: u64,
    fee: u64
) -> Result<u64> {
    // Return the original amount if the event was invalid or same
    if event_outcome == Outcome::Invalid || event_outcome == Outcome::Same {
        return Ok(order_amount)
    }

    // Return the original amount if one side wasn't entered
    if up_amount == 0 || down_amount == 0 {
        return Ok(order_amount)
    }

    // Losers get nothing
    if event_outcome != order_outcome {
        return Ok(0)
    }

    // Winners get their original amount back plus their earnings minus fees
    Ok(order_amount
        .checked_add(earned_amount)
        .ok_or(Error::OverflowError)?
        .checked_sub(fee)
        .ok_or(Error::OverflowError)?)
}

#[cfg(test)]
mod tests {
    use crate::instructions::settle_order::{get_amount_to_user, get_earned_amount, get_bps_amount};
    use crate::Outcome;

    #[test]
    fn earned_amount_up_only() {
        let value = get_earned_amount(
            100,
            0,
            Outcome::Up,
            10,
            Outcome::Up
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn earned_amount_down_only() {
        let value = get_earned_amount(
            0,
            100,
            Outcome::Up,
            10,
            Outcome::Up
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn earned_amount_same_outcome() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Same,
            10,
            Outcome::Up
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn earned_amount_invalid_outcome() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Invalid,
            10,
            Outcome::Up
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn earned_amount_order_outcome_incorrect() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Down,
            10,
            Outcome::Up
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn earned_amount_win_all() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Up,
            100,
            Outcome::Up
        ).unwrap();
        assert_eq!(100, value);
    }

    #[test]
    fn earned_amount_win_some() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Up,
            50,
            Outcome::Up
        ).unwrap();
        assert_eq!(50, value);
    }

    #[test]
    fn earned_amount_win_down() {
        let value = get_earned_amount(
            100,
            100,
            Outcome::Down,
            100,
            Outcome::Down
        ).unwrap();
        assert_eq!(100, value);
    }

    #[test]
    fn earned_amount_win_all_uneven_pool_up() {
        let value = get_earned_amount(
            200,
            100,
            Outcome::Up,
            100,
            Outcome::Up
        ).unwrap();
        assert_eq!(50, value);
    }

    #[test]
    fn earned_amount_win_all_uneven_pool_down() {
        let value = get_earned_amount(
            100,
            200,
            Outcome::Up,
            100,
            Outcome::Up
        ).unwrap();
        assert_eq!(200, value);
    }

    #[test]
    fn earned_amount_win_all_partial_uneven_pool_down() {
        let value = get_earned_amount(
            100,
            200,
            Outcome::Up,
            50,
            Outcome::Up
        ).unwrap();
        assert_eq!(100, value);
    }

    #[test]
    fn fees_zero() {
        let value = get_bps_amount(0, 100).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn fees_valid() {
        let value = get_bps_amount(100, 100).unwrap();
        assert_eq!(1, value);
    }

    #[test]
    fn fees_full() {
        let value = get_bps_amount(100, 10000).unwrap();
        assert_eq!(100, value);
    }

    #[test]
    fn amount_to_user_invalid() {
        let value = get_amount_to_user(
            Outcome::Invalid,
            100,
            100,
            Outcome::Up,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(10, value);
    }

    #[test]
    fn amount_to_user_same() {
        let value = get_amount_to_user(
            Outcome::Same,
            100,
            100,
            Outcome::Up,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(10, value);
    }

    #[test]
    fn amount_to_user_one_sided() {
        let value = get_amount_to_user(
            Outcome::Down,
            0,
            100,
            Outcome::Down,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(10, value);
    }

    #[test]
    fn amount_to_user_lose() {
        let value = get_amount_to_user(
            Outcome::Down,
            100,
            100,
            Outcome::Up,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn amount_to_user_lose_down() {
        let value = get_amount_to_user(
            Outcome::Up,
            100,
            100,
            Outcome::Down,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(0, value);
    }

    #[test]
    fn amount_to_user_win() {
        let value = get_amount_to_user(
            Outcome::Down,
            100,
            100,
            Outcome::Down,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(13, value);
    }

    #[test]
    fn amount_to_user_win_up() {
        let value = get_amount_to_user(
            Outcome::Up,
            100,
            100,
            Outcome::Up,
            10,
            5,
            2
        ).unwrap();
        assert_eq!(13, value);
    }
}