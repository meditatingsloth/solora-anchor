use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;
use crate::state::outcome::Outcome;

pub const EVENT_VERSION: u8 = 1;

pub const EVENT_SIZE: usize =
	8 + 1 + 1 + 32 + 32 + 32 + 32 + 4 + 8 + 8 + 4 + 8 + 8 + 2 + 16 + 16 + 4 + 4 + 1 + 4 + 256;

#[account]
pub struct Event {
	/// Bump seed used to generate the program address / authority
	pub bump: [u8; 1],
	pub version: u8,
	pub event_config: Pubkey,
	/// Clockwork thread that will perform the lock price update
	pub lock_thread: Pubkey,
	/// Clockwork thread that will perform the settle event update
	pub settle_thread: Pubkey,
	/// Account to receive fees
	pub fee_account: Pubkey,
	/// Fee rate in bps
	pub fee_bps: u32,
	/// Timestamp of when the event is open to orders
	pub start_time: i64,
	/// Timestamp of when the event is closed to new orders (start of waiting period)
	pub lock_time: i64,
	/// Seconds to wait after locking and before closing
	pub wait_period: u32,
	/// Price of the pyth feed at the time of lock
	pub lock_price: u64,
	/// Price of the pyth feed at the time of settlement
	pub settle_price: u64,
	/// Outcome of the event or 0 if not yet resolved
	pub outcome: Outcome,
	/// Store up and down bet amounts
	pub up_amount: u128,
	pub down_amount: u128,
	/// Store counts for UI
	pub up_count: u32,
	pub down_count: u32,
	/// Number of decimals to consider for price changes
	pub price_decimals: u8,
	/// Number of orders settled. Once it reaches the up_count + down_count it's safe to close the event
	pub orders_settled: u32
}

impl Event {
	pub fn auth_seeds<'a>(&'a self, start_time_bytes: &'a [u8]) -> [&'a[u8]; 4] {
		[
			b"event".as_ref(),
			self.event_config.as_ref(),
			start_time_bytes,
			self.bump.as_ref()
		]
	}
}