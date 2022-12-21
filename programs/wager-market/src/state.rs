use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

pub const EVENT_SIZE: usize = 8 + 1 + 32 + 32 + (4 + 200);

#[account]
pub struct Event {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub authority: Pubkey,
    // Bytes generated from sha256 of the event description
    pub id: [u8; 32],
    pub metadata_uri: String,
}

#[account(zero_copy)]
pub struct Order {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub authority: Pubkey,
    pub event: Pubkey,
    pub outcome: u8,
    pub bet_amount: u64,
    pub ask_bps: u32,
    pub fills: [Fill; 100]
}

#[zero_copy]
pub struct Fill {
    pub authority: Pubkey,
    pub outcome: u8,
    pub fill_amount: u64,
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