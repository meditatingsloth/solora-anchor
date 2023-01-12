use anchor_lang::prelude::*;
use clockwork_sdk::state::{Thread};
use pyth_sdk_solana::load_price_feed_from_account_info;
use crate::state::{Event, Outcome};
use crate::error::Error;

#[derive(Accounts)]
#[instruction(id: [u8; 32], outcome: Outcome)]
pub struct SettleEvent<'info> {
    #[account(
        mut,
        has_one = pyth_feed,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
        constraint = event.lock_price > 0 @ Error::LockPriceNotSet,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Safe due to event constraint
    pub pyth_feed: UncheckedAccount<'info>,

    #[account(
        signer,
        address = event.settle_thread,
        constraint = thread.id.eq("event_settle"),
        constraint = thread.authority == event.key()
    )]
    pub thread: Account<'info, Thread>,
}

pub fn settle_event<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
) -> Result<()> {
    let event = &mut ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if timestamp < event.lock_time + event.wait_period as i64  {
        return err!(Error::EventInWaitingPeriod);
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
            event.settled_price = price.price as u64;

            event.outcome = if event.settled_price == event.lock_price {
                Outcome::Invalid
            } else if event.settled_price > event.lock_price {
                Outcome::Up
            } else {
                Outcome::Down
            };
        }
    } else {
        msg!("No price found");
        event.outcome = Outcome::Invalid;
    }

    Ok(())
}