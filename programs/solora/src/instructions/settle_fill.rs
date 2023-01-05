use anchor_lang::prelude::*;
use crate::state::{Event, Fill, Order};
use crate::error::Error;
use crate::util::{is_default, transfer, transfer_sol};

#[derive(Accounts)]
#[instruction(index: u32)]
pub struct SettleFill<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &index.to_le_bytes()],
    bump,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = event.is_settled @ Error::EventNotSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn settle_fill<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleFill<'info>>,
    index: u32,
) -> Result<()> {
    let fill_index = ctx.accounts.order.get_fill_index(ctx.accounts.authority.key());
    if fill_index.is_none() {
        return err!(Error::FillNotFound);
    }

    let fill = &mut ctx.accounts.order.fills[fill_index.unwrap()];
    if fill.is_settled {
        return err!(Error::FillAlreadySettled);
    }

    fill.is_settled = true;
    // TODO: Remove fill from list and realloc, paying back the authority

    // TODO: Determine winner of wager
    if is_default(ctx.accounts.event.currency_mint) {
        transfer_sol(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.event.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            None,
            fill.fill_amount,
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
            fill.fill_amount,
        )?;
    }

    Ok(())
}