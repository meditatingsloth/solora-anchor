use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    #[msg("Swap account already in use")]
    AlreadyInUse,
}