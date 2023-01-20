use anchor_lang::prelude::*;
use clockwork_sdk::state::{Thread};
use pyth_sdk_solana::load_price_feed_from_account_info;
use crate::state::{Event, EventConfig, Outcome};
use crate::error::Error;
use crate::util::get_price_with_decimal_change;

#[derive(Accounts)]
pub struct SettleEvent<'info> {
    #[account(
        seeds = [
            b"event_config".as_ref(),
            event_config.authority.as_ref(),
            event_config.pyth_feed.as_ref(),
            event_config.currency_mint.as_ref()
        ],
        bump = event_config.bump[0],
        has_one = pyth_feed,
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
        has_one = settle_thread,
        constraint = event.outcome == Outcome::Undrawn @ Error::EventSettled,
        constraint = event.lock_price > 0 @ Error::LockPriceNotSet,
    )]
    pub event: Box<Account<'info, Event>>,

    /// CHECK: Safe due to event_config constraint
    pub pyth_feed: UncheckedAccount<'info>,

    #[account(
        signer,
        constraint = settle_thread.id.eq("event_settle"),
        constraint = settle_thread.authority == event.key()
    )]
    pub settle_thread: Account<'info, Thread>,
}

pub fn settle_event<'info>(
    ctx: Context<'_, '_, '_, 'info, SettleEvent<'info>>,
) -> Result<()> {
    let event = &mut ctx.accounts.event;

    let timestamp = Clock::get()?.unix_timestamp;
    if timestamp < event.lock_time + event.wait_period as i64  {
        return err!(Error::EventInWaitingPeriod);
    }

    // TODO: Delete settle thread when possible to delete from thread call

    let price_feed = load_price_feed_from_account_info(&ctx.accounts.pyth_feed.to_account_info()).unwrap();
    let price = price_feed.get_price_no_older_than(timestamp, 30);

    // Users will need to be able to withdraw funds if the price feed is not available, so set invalid outcome
    if price.is_some() {
        let price = price.unwrap();
        if price.price < 0 {
            msg!("Negative price: {}", price.price);
            event.outcome = Outcome::Invalid;
        } else {
            event.settle_price = get_price_with_decimal_change(
                price.price,
                price.expo,
                event.price_decimals
            )?;

            event.outcome = if event.settle_price == event.lock_price {
                Outcome::Same
            } else if event.settle_price > event.lock_price {
                Outcome::Up
            } else {
                Outcome::Down
            };
        }
    } else {
        msg!("No price found");
        event.outcome = Outcome::Invalid;
    }

    emit!(EventSettled {
        event_config: event.event_config,
        event: event.key(),
        settle_price: event.settle_price,
        outcome: event.outcome
    });
    Ok(())
}

#[event]
pub struct EventSettled {
    pub event_config: Pubkey,
    pub event: Pubkey,
    pub settle_price: u64,
    pub outcome: Outcome
}