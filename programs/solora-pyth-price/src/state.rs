mod event;
mod outcome;
mod order;
mod event_config;

pub use event::*;
pub use outcome::*;
pub use order::*;
pub use event_config::*;

pub const MAX_PRICE_DECIMALS: u8 = 4;