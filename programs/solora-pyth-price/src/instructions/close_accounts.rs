use crate::error::Error;
use crate::state::{Event, EventConfig, Outcome};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use clockwork_sdk::{
    ID as thread_program_ID,
    cpi::{
        thread_delete,
        ThreadDelete,
    },
    ThreadProgram,
};
use crate::util::{is_native_mint, transfer};

#[derive(Accounts)]
pub struct CloseAccounts<'info> {
    /// CHECK: Safe due to event config constraint
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,

    #[account(
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0],
        has_one = authority
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
        has_one = event_config,
        has_one = lock_thread,
        has_one = settle_thread,
        constraint = event.outcome != Outcome::Undrawn @ Error::EventNotSettled,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Safe due to event constraint
    #[account(mut)]
    pub lock_thread: UncheckedAccount<'info>,

    /// CHECK: Safe due to event constraint
    #[account(mut)]
    pub settle_thread: UncheckedAccount<'info>,

    #[account(address = thread_program_ID)]
    pub clockwork: Program<'info, ThreadProgram>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn close_accounts<'info>(ctx: Context<'_, '_, '_, 'info, CloseAccounts<'info>>) -> Result<()> {
    let event = &ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if event.lock_time + event.wait_period as i64 > timestamp {
        return err!(Error::EventNotSettled);
    }

    let start_time_bytes = &event.start_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(start_time_bytes);

    if !ctx.accounts.lock_thread.data_is_empty() {
        thread_delete(CpiContext::new_with_signer(
            ctx.accounts.clockwork.to_account_info(),
            ThreadDelete {
                authority: event.to_account_info(),
                close_to: ctx.accounts.authority.to_account_info(),
                thread: ctx.accounts.lock_thread.to_account_info(),
            },
            &[&auth_seeds]
        ))?;
    }

    if !ctx.accounts.settle_thread.data_is_empty() {
        thread_delete(CpiContext::new_with_signer(
            ctx.accounts.clockwork.to_account_info(),
            ThreadDelete {
                authority: event.to_account_info(),
                close_to: ctx.accounts.authority.to_account_info(),
                thread: ctx.accounts.settle_thread.to_account_info(),
            },
            &[&auth_seeds]
        ))?;
    }

    // Close the event if all orders have been settled
    if event.up_count + event.down_count == event.orders_settled {
        // Empty/close the currency account as well if not using native mint
        if !is_native_mint(ctx.accounts.event_config.currency_mint) {
            let remaining_accounts = &mut ctx.remaining_accounts.iter();
            let currency_mint = next_account_info(remaining_accounts)?;
            let event_currency_account = next_account_info(remaining_accounts)?;

            let token_account =
                Account::<'info, TokenAccount>::try_from(event_currency_account)?;
            if token_account.amount > 0 {
                let authority_currency_account = next_account_info(remaining_accounts)?;
                let token_program = next_account_info(remaining_accounts)?;
                let ata_program = next_account_info(remaining_accounts)?;
                let rent = next_account_info(remaining_accounts)?;

                let start_time_bytes = &event.start_time.to_le_bytes();
                let auth_seeds = event.auth_seeds(start_time_bytes);

                transfer(
                    &event.to_account_info(),
                    &ctx.accounts.authority.to_account_info(),
                    event_currency_account.into(),
                    authority_currency_account.into(),
                    currency_mint.into(),
                    Option::from(&ctx.accounts.authority.to_account_info()),
                    ata_program.into(),
                    token_program.into(),
                    &ctx.accounts.system_program.to_account_info(),
                    rent.into(),
                    Some(&auth_seeds),
                    None,
                    token_account.amount
                )?;
            }

            token_account.close(ctx.accounts.authority.to_account_info())?;
        }

        event.close(ctx.accounts.authority.to_account_info())?;
    }

    Ok(())
}
