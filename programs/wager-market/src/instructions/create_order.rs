use anchor_lang::prelude::*;
use crate::state::{Event, Fill, Order, ORDER_SIZE};
use crate::error::Error;
use crate::util::{assert_is_ata, is_default, transfer, transfer_sol};

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

    #[account(mut)]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateOrder<'info>>,
    outcome: u8,
    bet_amount: u64,
    ask_bps: u32,
    expiry: Option<i64>
) -> Result<()> {
    if ctx.accounts.event.is_settled {
        return err!(Error::EventSettled);
    }

    if is_default(ctx.accounts.event.currency_mint) {
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.event.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            bet_amount,
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
            bet_amount,
        )?;
    }

    let order = &mut ctx.accounts.order;
    order.index = ctx.accounts.event.order_index;
    order.authority = ctx.accounts.authority.key();
    order.event = ctx.accounts.event.key();
    order.outcome = outcome;
    order.bet_amount = bet_amount;
    order.ask_bps = ask_bps;
    order.fills = Vec::new();

    if expiry.is_some() {
        let timestamp = Clock::get()?.unix_timestamp;
        if expiry.unwrap() <= timestamp {
            return err!(Error::InvalidExpiry);
        }
        order.expiry = expiry.unwrap();
    } else {
        order.expiry = -1;
    }

    ctx.accounts.event.order_index += 1;

    Ok(())
}