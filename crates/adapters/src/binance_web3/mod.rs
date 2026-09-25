//! Binance Web3 RWA adapter for the bStock catalog port.

mod auth;
mod catalog;
mod mapping;

pub use auth::BinanceConfigurationError;
pub use catalog::BinanceRwaCatalog;

const BSTOCK_PLATFORM_ID: &str = "bstock";
