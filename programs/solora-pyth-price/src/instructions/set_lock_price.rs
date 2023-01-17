use crate::error::Error;
use crate::state::{Event, EventConfig, Outcome};
use crate::util::get_price_with_decimal_change;
use anchor_lang::prelude::*;
use clockwork_sdk::state::Thread;
use pyth_sdk_solana::load_price_feed_from_account_info;

#[derive(Accounts)]
pub struct SetLockPrice<'info> {
    #[account(
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0],
        has_one = pyth_feed
    )]
    pub event_config: Box<Account<'info, EventConfig>>,

    #[account(
        mut,
        seeds = [
            b"event".as_ref(),
            event_config.key().as_ref(),
            &event.lock_time.to_le_bytes()
        ],
        bump = event.bump[0],
        has_one = event_config,
        has_one = lock_thread,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
        constraint = event.lock_price == 0 @ Error::LockPriceSet,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Safe due to event_config constraint
    pub pyth_feed: UncheckedAccount<'info>,

    #[account(
        signer,
        constraint = lock_thread.id.eq("event_lock"),
        constraint = lock_thread.authority == event.key()
    )]
    pub lock_thread: Account<'info, Thread>,
}

pub fn set_lock_price<'info>(ctx: Context<'_, '_, '_, 'info, SetLockPrice<'info>>) -> Result<()> {
    let event = &mut ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if event.lock_time > timestamp {
        return err!(Error::EventNotLocked);
    }

    // TODO: Delete lock thread

    // TODO: Close event account if there are no bets
    // TODO: Delete settle thread if there are no bets

    // TODO: Set invalid outcome if only one side has bets

    let price_feed =
        load_price_feed_from_account_info(&ctx.accounts.pyth_feed.to_account_info()).unwrap();
    let price = price_feed.get_price_no_older_than(timestamp, 30);

    // Users will need to be able to withdraw funds if the price feed is not available, so set invalid outcome
    if price.is_some() {
        let price = price.unwrap();
        if price.price < 0 {
            msg!("Negative price: {}", price.price);
            event.outcome = Outcome::Invalid;
        } else {
            event.lock_price =
                get_price_with_decimal_change(price.price, price.expo, event.price_decimals)?;
        }
    } else {
        msg!("No price found");
        event.outcome = Outcome::Invalid;
    }

    emit!(EventLocked {
        event_config: event.event_config,
        event: event.key(),
        lock_price: event.lock_price,
        up_amount: event.up_amount,
        down_amount: event.down_amount,
        up_count: event.up_count,
        down_count: event.down_count,
    });

    Ok(())
}

#[event]
pub struct EventLocked {
    pub event_config: Pubkey,
    pub event: Pubkey,
    pub lock_price: u64,
    pub up_amount: u128,
    pub down_amount: u128,
    pub up_count: u32,
    pub down_count: u32,
}
