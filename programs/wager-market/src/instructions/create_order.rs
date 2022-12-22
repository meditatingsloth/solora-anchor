use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::state::{Event, Fill, Order, ORDER_SIZE};
use crate::error::Error;
use crate::util::{assert_is_ata, is_default};

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

    #[account()]
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

    if !is_default(ctx.accounts.event.currency_mint) {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let user_currency_account = next_account_info(remaining_accounts)?;
        assert_is_ata(
            user_currency_account,
            &ctx.accounts.authority.key(),
            &ctx.accounts.event.currency_mint
        )?;
    }

    let order = &mut ctx.accounts.order;
    order.authority = ctx.accounts.authority.key();
    order.event = ctx.accounts.event.key();
    order.outcome = outcome;
    order.bet_amount = bet_amount;
    order.ask_bps = ask_bps;
    order.fills = Vec::new();

    if expiry.is_some() {
        order.expiry = expiry.unwrap();
    } else {
        order.expiry = -1;
    }


    Ok(())
}