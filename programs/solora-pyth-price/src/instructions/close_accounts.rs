use crate::error::Error;
use crate::state::{Event, EventConfig, Outcome};
use anchor_lang::prelude::*;
use clockwork_sdk::{
    ID as thread_program_ID,
    cpi::{
        thread_delete,
        ThreadDelete,
    },
    state::{Thread},
    ThreadProgram,
};

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

    #[account(
        mut,
        constraint = lock_thread.id.eq("event_lock"),
        constraint = lock_thread.authority == event.key()
    )]
    pub lock_thread: Account<'info, Thread>,

    #[account(
        mut,
        constraint = settle_thread.id.eq("event_settle"),
        constraint = settle_thread.authority == event.key()
    )]
    pub settle_thread: Account<'info, Thread>,

    #[account(address = thread_program_ID)]
    pub clockwork: Program<'info, ThreadProgram>,
}

pub fn close_accounts<'info>(ctx: Context<'_, '_, '_, 'info, CloseAccounts<'info>>) -> Result<()> {
    let event = &mut ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if event.lock_time + event.wait_period as i64 > timestamp {
        return err!(Error::EventNotSettled);
    }

    let start_time_bytes = &event.start_time.to_le_bytes();
    let auth_seeds = event.auth_seeds(start_time_bytes);

    thread_delete(CpiContext::new_with_signer(
        ctx.accounts.clockwork.to_account_info(),
        ThreadDelete {
            authority: event.to_account_info(),
            close_to: ctx.accounts.authority.to_account_info(),
            thread: ctx.accounts.lock_thread.to_account_info(),
        },
        &[&auth_seeds]
    ))?;

    thread_delete(CpiContext::new_with_signer(
        ctx.accounts.clockwork.to_account_info(),
        ThreadDelete {
            authority: event.to_account_info(),
            close_to: ctx.accounts.authority.to_account_info(),
            thread: ctx.accounts.settle_thread.to_account_info(),
        },
        &[&auth_seeds]
    ))?;

    // Close the event if there were no bets
    if event.up_amount == 0 &&
        event.down_amount == 0 {
        event.close(ctx.accounts.authority.to_account_info())?;
    }

    Ok(())
}
