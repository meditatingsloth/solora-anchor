use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    /// 0
    #[msg("Event has already been settled")]
    EventSettled,
    #[msg("An invalid outcome was chosen")]
    InvalidOutcome,
    #[msg("There was a calculation overflow")]
    OverflowError,
    #[msg("The fill amount is too large")]
    FillAmountTooLarge,
    #[msg("The expiry date has passed")]
    InvalidExpiry,

    /// 5
    #[msg("The order has expired")]
    OrderExpired,
    #[msg("The user already has an existing fill for this order")]
    UserAlreadyFilled,
    #[msg("Event has not been settled")]
    EventNotSettled,
    #[msg("A fill was not found for this user")]
    FillNotFound,
    #[msg("The fill has already been settled")]
    FillAlreadySettled,

    /// 10
    #[msg("The order has already been filled")]
    OrderFilled,
    #[msg("The amount to remove is more than remaining ask")]
    AmountLargerThanRemainingAsk,
    #[msg("The fill authority does not match")]
    InvalidFillAuthority,
    #[msg("Invalid close time")]
    InvalidCloseTime,
    #[msg("Event has been closed")]
    EventClosed,
}