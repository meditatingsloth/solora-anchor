use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    /// 0
    #[msg("Event has already been settled")]
    EventSettled,
    #[msg("An invalid outcome was chosen")]
    InvalidOutcome,
    #[msg("There was a calculation overflow")]
    CalculationOverflow,
    #[msg("The fill amount is too large")]
    FillAmountTooLarge,
    #[msg("The expiry date has passed")]
    InvalidExpiry,

    /// 5
    #[msg("The order has expired")]
    OrderExpired,
    #[msg("The user already has an existing fill for this order")]
    UserAlreadyFilled,
}