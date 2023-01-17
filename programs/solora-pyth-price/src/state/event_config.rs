use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub const EVENT_CONFIG_VERSION: u8 = 1;

pub const EVENT_CONFIG_SIZE: usize =
	8 + 1 + 1 + 32 + 32 + 32 + 4 + 8 + 256;

#[account]
pub struct EventConfig {
	/// Bump seed used to generate the program address / authority
	pub bump: [u8; 1],
	pub version: u8,
	/// Owner of the configuration
	pub authority: Pubkey,
	/// Pyth price feed account to fetch prices from
	pub pyth_feed: Pubkey,
	/// SPL token mint or native mint for SOL for the pool bets
	pub currency_mint: Pubkey,
	/// Number of seconds between start/lock/settle
	pub interval_seconds: u32,
	/// Unix timestamp of the next time an event should start for this config
	pub next_event_start: i64
}

impl EventConfig {
	/// Seeds are unique to authority/pyth feed/currency mint combinations
	pub fn auth_seeds<'a>(&'a self) -> [&'a[u8]; 5] {
		[
			b"event_config".as_ref(),
			self.authority.as_ref(),
			self.pyth_feed.as_ref(),
			self.currency_mint.as_ref(),
			self.bump.as_ref()
		]
	}
}