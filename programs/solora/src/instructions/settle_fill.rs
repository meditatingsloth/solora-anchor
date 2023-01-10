use anchor_lang::prelude::*;
use crate::state::{Event, Order};
use crate::error::Error;
use crate::util::{is_default, transfer};

#[derive(Accounts)]
#[instruction(order_index: u32, fill_index: u32)]
pub struct SettleFill<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Safe due to order constraint
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Safe due to constraint
    #[account(
    mut,
    constraint = order.fills[fill_index as usize].authority == authority.key() @ Error::InvalidFillAuthority,
    )]
    pub fill_authority: UncheckedAccount<'info>,

    /// CHECK: Safe due to event constraint
    #[account(mut)]
    pub fee_account: UncheckedAccount<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &order_index.to_le_bytes()],
    bump,
    has_one = authority
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = event.outcome != 0 @ Error::EventNotSettled,
    has_one = fee_account
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn settle_fill<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleFill<'info>>,
    order_index: u32,
    fill_index: u32
) -> Result<()> {
    if fill_index > ctx.accounts.order.fills.len() as u32 - 1 {
        return err!(Error::FillNotFound);
    }

    let order = &mut ctx.accounts.order;
    let order_clone = order.clone();
    let fill = &order.fills[fill_index as usize];
    if fill.is_settled {
        return err!(Error::FillAlreadySettled);
    }

    let is_native = is_default(order_clone.currency_mint);
    let index_bytes = &order_index.to_le_bytes();
    let seeds = order.auth_seeds(index_bytes);
    let auth_seeds = seeds.as_ref();

    // TODO: Remove fill from list and realloc, paying back the authority

    // TODO: Determine winner of wager
    let is_order_winner = order.outcome == ctx.accounts.event.outcome;
    let is_draw = !is_order_winner && fill.outcome != ctx.accounts.event.outcome;

    let order_obligation = (fill.amount as u128)
        .checked_mul(10000 as u128).unwrap()
        .checked_div(order.ask_bps as u128).unwrap() as u64;

    let mut amount_to_order_authority = if is_order_winner {
        order_obligation.checked_add(fill.amount).unwrap()
    } else if is_draw {
        order_obligation
    } else {
        0
    };

    let mut amount_to_fill_authority = if is_order_winner {
        0
    } else if is_draw {
        fill.amount
    } else {
        order_obligation.checked_add(fill.amount).unwrap()
    };

    let fee = if is_order_winner {
        (fill.amount as u128).checked_mul(ctx.accounts.event.fee_bps as u128).unwrap()
            .checked_div(10000 as u128).unwrap() as u64
    } else if is_draw {
        0
    } else {
        (order_obligation as u128).checked_mul(ctx.accounts.event.fee_bps as u128).unwrap()
            .checked_div(10000 as u128).unwrap() as u64
    };

    let remaining_accounts = &mut ctx.remaining_accounts.iter();
    let (
        currency_mint,
        order_currency_account,
        authority_currency_account,
        fee_currency_account,
        fill_authority_currency_account,
        token_program,
        ata_program,
        rent
    ) = if is_native {
        (
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?),
            Option::from(next_account_info(remaining_accounts)?)
        )
    } else {
        (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None
        )
    };

    if amount_to_order_authority > 0 {
        if fee > 0 {
            amount_to_order_authority = amount_to_order_authority.checked_sub(fee).unwrap();
        }

        if amount_to_order_authority > 0 {
            transfer(
                &order_clone.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                order_currency_account,
                authority_currency_account,
                currency_mint,
                Option::from(&ctx.accounts.payer.to_account_info()),
                ata_program,
                token_program,
                &ctx.accounts.system_program.to_account_info(),
                rent,
                auth_seeds.into(),
                None,
                amount_to_order_authority,
            )?;
        }
    }

    if amount_to_fill_authority > 0 {
        if fee > 0 {
            amount_to_fill_authority = amount_to_fill_authority.checked_sub(fee).unwrap();
        }

        if amount_to_fill_authority > 0 {
            transfer(
                &order_clone.to_account_info(),
                &ctx.accounts.fill_authority.to_account_info(),
                order_currency_account,
                fill_authority_currency_account,
                currency_mint,
                Option::from(&ctx.accounts.payer.to_account_info()),
                ata_program,
                token_program,
                &ctx.accounts.system_program.to_account_info(),
                rent,
                auth_seeds.into(),
                None,
                amount_to_fill_authority,
            )?;
        }
    }

    if fee > 0 {
        transfer(
            &order_clone.to_account_info(),
            &ctx.accounts.fee_account.to_account_info(),
            order_currency_account,
            fee_currency_account,
            currency_mint,
            Option::from(&ctx.accounts.payer.to_account_info()),
            ata_program,
            token_program,
            &ctx.accounts.system_program.to_account_info(),
            rent,
            auth_seeds.into(),
            None,
            fee,
        )?;
    }

    let fill = &mut order.fills[fill_index as usize];
    fill.is_settled = true;

    Ok(())
}