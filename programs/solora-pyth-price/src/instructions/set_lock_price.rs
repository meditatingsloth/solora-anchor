use crate::error::Error;
use crate::state::{Event, Outcome};
use clockwork_sdk::state::{Thread};
use anchor_lang::prelude::*;
use pyth_sdk_solana::load_price_feed_from_account_info;

#[derive(Accounts)]
pub struct SetLockPrice<'info> {
    #[account(
        mut,
        has_one = pyth_feed,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
        constraint = event.lock_price == 0 @ Error::LockPriceSet,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Safe due to event constraint
    pub pyth_feed: UncheckedAccount<'info>,

    #[account(
        signer,
        address = event.lock_thread,
        constraint = thread.id.eq("event_lock"),
        constraint = thread.authority == event.key()
    )]
    pub thread: Account<'info, Thread>,
}

pub fn set_lock_price<'info>(
    ctx: Context<'_, '_, '_, 'info, SetLockPrice<'info>>,
) -> Result<()> {
    let event = &mut ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if event.lock_time > timestamp {
        return err!(Error::EventNotLocked);
    }

    let price_feed = load_price_feed_from_account_info(&ctx.accounts.pyth_feed.to_account_info()).unwrap();
    let price = price_feed.get_price_no_older_than(timestamp, 30);

    // Users will need to be able to withdraw funds if the price feed is not available, so set invalid outcome
    if price.is_some() {
        let price = price.unwrap();
        if price.price < 0 {
            msg!("Negative price: {}", price.price);
            event.outcome = Outcome::Invalid;
        } else {
            event.lock_price = price.price as u64;
        }
    } else {
        msg!("No price found");
        event.outcome = Outcome::Invalid;
    }

    Ok(())
}
