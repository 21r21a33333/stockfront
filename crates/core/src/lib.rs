//! Deterministic, I/O-free Stockfront domain logic.

pub mod baskets;
pub mod bstock_borrowing;
pub mod bstock_market;
pub mod bstock_trade;
pub mod carry;
pub mod liquidity;
pub mod managed_portfolios;
pub mod prepared_action;
pub mod recipes;
pub mod strategy_composer;
pub mod wallet_portfolio;

mod serialization;
