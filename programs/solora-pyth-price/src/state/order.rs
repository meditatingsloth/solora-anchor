use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;
use crate::Outcome;

pub const ORDER_SIZE: usize = 8 + 1 + 1 + 4 + 32 + 32 + 1 + 8 + 32 + 4 + 8 + 8 + 4;

#[account]
pub struct Order {
	/// Bump seed used to generate the program address / authority
	pub bump: [u8; 1],
	pub version: u8,
	pub authority: Pubkey,
	pub event: Pubkey,
	pub outcome: Outcome,
	pub amount: u64
}

impl Order {
	pub fn space() -> usize {
		ORDER_SIZE
	}

	pub fn auth_seeds<'a>(&'a self) -> [&'a [u8]; 4] {
		[
			b"order".as_ref(),
			self.event.as_ref(),
			self.authority.as_ref(),
			self.bump.as_ref()
		]
	}
}
