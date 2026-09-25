//! Binance Web3 adapters for bStock market and wallet ports.

mod auth;
mod balances;
mod catalog;
mod client;
mod mapping;

pub use auth::BinanceConfigurationError;
pub use client::BinanceWeb3Client;

const BSTOCK_PLATFORM_ID: &str = "bstock";
