use anchor_lang::prelude::*;
use solana_program::pubkey;

pub fn key_is_default(key1: Pubkey) -> bool {
    return key1 == pubkey::Pubkey::default();
}