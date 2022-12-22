use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    #[msg("Event has already been settled")]
    EventSettled,
    #[msg("An invalid outcome was chosen")]
    InvalidOutcome,
    #[msg("There was a calculation overflow")]
    CalculationOverflow,
    #[msg("The fill amount is too large")]
    FillAmountTooLarge,
    #[msg("The expiry date has passed")]
    InvalidExpiry
}