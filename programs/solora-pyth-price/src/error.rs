use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    /// 0
    #[msg("Event has already been settled")]
    EventSettled,
    #[msg("An invalid outcome was chosen")]
    InvalidOutcome,
    #[msg("Overflow error")]
    OverflowError,
    #[msg("The expiry date has passed")]
    InvalidExpiry,
    #[msg("The order has expired")]
    OrderExpired,

    /// 5
    #[msg("Event has not been settled")]
    EventNotSettled,
    #[msg("A fill was not found for this user")]
    FillNotFound,
    #[msg("The fill has already been settled")]
    FillAlreadySettled,
    #[msg("The order has already been filled")]
    OrderFilled,
    #[msg("The amount to remove is more than remaining ask")]
    AmountLargerThanRemainingAsk,

    /// 10
    #[msg("The fill authority does not match")]
    InvalidFillAuthority,
    #[msg("Invalid lock time")]
    InvalidLockTime,
    #[msg("Event has been locked")]
    EventLocked,
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("The event has not been locked yet")]
    EventNotLocked,

    /// 15
    #[msg("The lock price has already been set")]
    LockPriceSet,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("The lock price has not been set")]
    LockPriceNotSet,
    #[msg("The event is still in the waiting period")]
    EventInWaitingPeriod,
    #[msg("Invalid interval")]
    InvalidInterval,

    /// 20
    #[msg("The event has not started")]
    EventNotStarted,
}