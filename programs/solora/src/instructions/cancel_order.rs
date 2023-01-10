use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{CloseAccount};
use crate::state::{Event, Order};
use crate::error::Error;
use crate::util::{is_default, transfer};

#[derive(Accounts)]
#[instruction(index: u32, amount: u64)]
pub struct CancelOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    mut,
    seeds = [b"order".as_ref(), event.key().as_ref(), &index.to_le_bytes()],
    bump,
    has_one = authority,
    constraint = order.remaining_ask > 0 @ Error::OrderFilled,
    constraint = amount <= order.remaining_ask @ Error::AmountLargerThanRemainingAsk,
    )]
    pub order: Box<Account<'info, Order>>,

    #[account(
    mut,
    constraint = event.outcome == 0 @ Error::EventSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
}

pub fn cancel_order<'info>(
    ctx: Context<'_, '_, '_, 'info, CancelOrder<'info>>,
    index: u32,
    amount: u64
) -> Result<()> {
    let is_native = is_default(ctx.accounts.order.currency_mint);

    // No fills so we can return funds and close order account(s)
    if ctx.accounts.order.fills.len() == 0 {
        if !is_native {
            let remaining_accounts = &mut ctx.remaining_accounts.iter();
            let currency_mint = next_account_info(remaining_accounts)?;
            let order_currency_account = next_account_info(remaining_accounts)?;
            let user_currency_account = next_account_info(remaining_accounts)?;
            let token_program = next_account_info(remaining_accounts)?;
            let ata_program = next_account_info(remaining_accounts)?;
            let rent = next_account_info(remaining_accounts)?;
            let index_bytes = &index.to_le_bytes();
            let seeds = ctx.accounts.order.auth_seeds(index_bytes);
            let auth_seeds = seeds.as_ref();

            transfer(
                &ctx.accounts.order.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                order_currency_account.into(),
                user_currency_account.into(),
                currency_mint.into(),
                Option::from(&ctx.accounts.authority.to_account_info()),
                ata_program.into(),
                token_program.into(),
                &ctx.accounts.system_program.to_account_info(),
                rent.into(),
                auth_seeds.into(),
                None,
                ctx.accounts.order.amount,
            )?;

            token::close_account(
                CpiContext::new(
                    token_program.to_account_info(),
                    CloseAccount {
                        account: order_currency_account.to_account_info(),
                        destination: ctx.accounts.authority.to_account_info(),
                        authority: ctx.accounts.order.to_account_info(),
                    },
                ).with_signer(&[auth_seeds]),
            )?;
        }

        ctx.accounts.order.close(ctx.accounts.authority.to_account_info())?;
    } else {
        // TODO: Reduce bet amount if partially filled
    }

    Ok(())
}