use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub const EVENT_SIZE: usize = 8 + 1 + 1 + 4 + 32 + 32 + 1 + 32 + 4 + (4 + 200);

#[account]
pub struct Event {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub version: u8,
    /// Index to use for the next order created on this event
    pub order_index: u32,
    pub authority: Pubkey,
    /// Bytes generated from sha256 of the event description
    pub id: [u8; 32],
    /// Outcome of the event or 0 if not yet resolved
    pub outcome: u8,
    /// Account to receive fees
    pub fee_account: Pubkey,
    /// Fee rate in bps
    pub fee_bps: u32,
    pub metadata_uri: String,
}

impl Event {
    pub fn auth_seeds(&self) -> [&[u8]; 3] {
        [
            b"event".as_ref(),
            self.id.as_ref(),
            self.bump.as_ref()
        ]
    }
}

pub const ORDER_SIZE: usize = 8 + 1 + 1 + 4 + 32 + 32 + 1 + 8 + 32 + 4 + 8 + 8 + 4;

#[account]
pub struct Order {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub version: u8,
    /// Index of this order within the event. Allows a user to create multiple orders.
    pub index: u32,
    pub authority: Pubkey,
    pub event: Pubkey,
    pub outcome: u8,
    pub amount: u64,
    /// SPL token mint or native mint for SOL
    pub currency_mint: Pubkey,
    pub ask_bps: u32,
    pub remaining_ask: u64,
    /// Expires any remaining bet_amount after this timestamp or 0 if never expires
    pub expiry: i64,
    /// Used instead of separate accounts to reduce number of accounts needed when settling
    pub fills: Vec<Fill>
}

impl Order {
    pub fn space(fill_len: usize) -> usize {
        ORDER_SIZE + (fill_len * FILL_SIZE)
    }

    pub fn get_fill_index(&self, authority: Pubkey) -> Option<usize> {
        self.fills.iter().position(|fill| fill.authority == authority)
    }

    pub fn auth_seeds<'a>(&'a self, index_bytes: &'a [u8]) -> [&'a [u8]; 4] {
        [
            b"order".as_ref(),
            self.event.as_ref(),
            index_bytes,
            self.bump.as_ref()
        ]
    }
}

pub const FILL_SIZE: usize = 4 + 32 + 1 + 8 + 1;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Fill {
    pub index: u32,
    pub authority: Pubkey,
    pub outcome: u8,
    pub amount: u64,
    pub is_settled: bool
}