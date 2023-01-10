use anchor_lang::prelude::*;
use crate::state::{Event, Order, ORDER_SIZE};
use crate::error::Error;
use crate::util::{transfer, transfer_sol};

#[derive(Accounts)]
pub struct CreateOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    init,
    seeds = [b"order".as_ref(), event.key().as_ref(), &event.order_index.to_le_bytes()],
    bump,
    space = ORDER_SIZE,
    payer = authority,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = event.outcome == 0 @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
    outcome: u8,
    amount: u64,
    ask_bps: u32,
    expiry: i64
) -> Result<()> {
    if expiry != 0 {
        let timestamp = Clock::get()?.unix_timestamp;
        if expiry <= timestamp {
            return err!(Error::InvalidExpiry);
        }
    }

    if outcome == 0 {
        return err!(Error::InvalidOutcome);
    }

    let order = &mut ctx.accounts.order;
    order.bump = [*ctx.bumps.get("order").unwrap()];
    order.index = ctx.accounts.event.order_index;
    order.authority = ctx.accounts.authority.key();
    order.event = ctx.accounts.event.key();
    order.outcome = outcome;
    order.amount = amount;
    order.ask_bps = ask_bps;
    order.remaining_ask = (amount as u128)
        .checked_mul(ask_bps as u128).unwrap()
        .checked_div(10000 as u128).unwrap() as u64;
    order.expiry = expiry;
    order.fills = Vec::new();

    // If there are remaining_accounts populated we're using an alt currency mint
    if ctx.remaining_accounts.len() == 0 {
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &order.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            amount,
        )?;

        order.currency_mint = Pubkey::default();
    } else {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        let order_currency_account = next_account_info(remaining_accounts)?;
        let user_currency_account = next_account_info(remaining_accounts)?;
        let token_program = next_account_info(remaining_accounts)?;
        let ata_program = next_account_info(remaining_accounts)?;
        let rent = next_account_info(remaining_accounts)?;

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

        order.currency_mint = currency_mint.key();
    }

    ctx.accounts.event.order_index = ctx.accounts.event.order_index.checked_add(1)
        .ok_or(Error::CalculationOverflow)?;

    Ok(())
}