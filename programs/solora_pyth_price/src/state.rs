use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub const EVENT_SIZE: usize = 8 + 1 + 1 + 32 + 32 + 32 + 32 + 32 + 4 + 8 + 4 + 32 + 8 + 8 + 2 + 16 + 16 + 8 + 8 + 256;

#[account]
pub struct Event {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub version: u8,
    pub authority: Pubkey,
    /// Clockwork thread that will perform the lock price update
    pub lock_thread: Pubkey,
    /// Clockwork thread that will perform the settle event update
    pub settle_thread: Pubkey,
    /// Bytes generated from sha256 of the event description
    pub pyth_feed: Pubkey,
    /// Account to receive fees
    pub fee_account: Pubkey,
    /// Fee rate in bps
    pub fee_bps: u32,
    /// Timestamp of when the event is closed to new orders (start of waiting period)
    pub lock_time: i64,
    /// Seconds to wait after locking and before closing
    pub wait_period: u32,
    /// SPL token mint or native mint for SOL for the pool bets
    pub currency_mint: Pubkey,
    /// Price of the pyth feed at the time of lock
    pub lock_price: u64,
    /// Price of the pyth feed at the time of settlement
    pub settled_price: u64,
    /// Outcome of the event or 0 if not yet resolved
    pub outcome: Outcome,
    /// Store up and down bet amounts
    pub up_amount: u128,
    pub down_amount: u128,
    /// Store counts for UI
    pub up_count: u32,
    pub down_count: u32
}

impl Event {
    pub fn auth_seeds<'a>(&'a self, lock_time_bytes: &'a [u8]) -> [&'a[u8]; 6] {
        [
            b"event".as_ref(),
            self.pyth_feed.as_ref(),
            self.fee_account.as_ref(),
            self.currency_mint.as_ref(),
            lock_time_bytes,
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
    pub authority: Pubkey,
    pub event: Pubkey,
    pub outcome: Outcome,
    pub amount: u64
}

impl Order {
    pub fn space(fill_len: usize) -> usize {
        ORDER_SIZE + (fill_len * FILL_SIZE)
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

pub const FILL_SIZE: usize = 4 + 32 + 1 + 8 + 1;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Fill {
    pub index: u32,
    pub authority: Pubkey,
    pub outcome: u8,
    pub amount: u64,
    pub is_settled: bool
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Debug)]
pub enum Outcome {
    Undrawn,
    Invalid,
    Up,
    Down,
}