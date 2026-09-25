//! Inbound HTTP adapter for the bStock market slice.

mod error;
mod handlers;
mod primitives;
mod request;

use std::sync::Arc;

use axum::{Router, routing::get};
use stockfront_core::bstock_market::BstockMarketService;
use stockfront_core::wallet_portfolio::WalletPortfolioService;

pub fn bstock_router(service: Arc<BstockMarketService>) -> Router {
    Router::new()
        .route("/v1/bstocks", get(handlers::bstocks::list_bstocks))
        .route(
            "/v1/bstocks/{chain_id}/{contract}",
            get(handlers::bstocks::get_bstock),
        )
        .with_state(service)
}

pub fn wallet_router(service: Arc<WalletPortfolioService>) -> Router {
    Router::new()
        .route(
            "/v1/wallets/{address}/portfolio",
            get(handlers::wallets::portfolio),
        )
        .with_state(service)
}
