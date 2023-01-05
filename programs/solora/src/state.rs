use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub const EVENT_SIZE: usize = 8 + 1 + 4 + 32 + 32 + 1 + 32 + (4 + 200);

#[account]
pub struct Event {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    /// Index to use for the next order created on this event
    pub order_index: u32,
    pub authority: Pubkey,
    /// Bytes generated from sha256 of the event description
    pub id: [u8; 32],
    pub is_settled: bool,
    pub currency_mint: Pubkey,
    pub metadata_uri: String,
}

pub const ORDER_SIZE: usize = 8 + 1 + 4 + 32 + 32 + 1 + 8 + 4 + 8 + 4;

#[account]
pub struct Order {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub index: u32,
    pub authority: Pubkey,
    pub event: Pubkey,
    pub outcome: u8,
    pub bet_amount: u64,
    pub ask_bps: u32,
    /// Expires any remaining bet_amount after this timestamp or -1 if never expires
    pub expiry: i64,
    pub fills: Vec<Fill>
}

impl Order {
    pub fn space(fill_len: usize) -> usize {
        ORDER_SIZE + (fill_len * FILL_SIZE)
    }

    pub fn get_fill_index(&self, authority: Pubkey) -> Option<usize> {
        self.fills.iter().position(|fill| fill.authority == authority)
    }

    pub fn get_remaining_ask(&self) -> Option<u64> {
        let total_filled = self.fills.iter().fold(0 as u64, |acc, fill| acc + fill.fill_amount);
        msg!("total_filled: {}", total_filled);
        let total_ask = (self.bet_amount as u128)
            .checked_mul(self.ask_bps as u128).unwrap()
            .checked_div(10000 as u128).unwrap() as u64;
        msg!("total_ask: {}", total_ask);
        let remaining_ask = total_ask.checked_sub(total_filled).unwrap();
        msg!("remaining_ask: {}", remaining_ask);
        return Some(remaining_ask);
    }
}

pub const FILL_SIZE: usize = 32 + 1 + 8 + 1;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Fill {
    pub authority: Pubkey,
    pub outcome: u8,
    pub fill_amount: u64,
    pub is_settled: bool
}

pub trait AuthSeeds<const SIZE: usize> {
    fn auth_seeds(&self) -> [&[u8]; SIZE];
}

impl AuthSeeds<3> for Event {
    fn auth_seeds(&self) -> [&[u8]; 3] {
        [
            b"event".as_ref(),
            self.id.as_ref(),
            self.bump.as_ref()
        ]
    }
}