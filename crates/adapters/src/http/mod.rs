//! Inbound HTTP adapter for the bStock market slice.

mod error;
mod handlers;
mod primitives;
mod request;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use stockfront_core::{
    bstock_market::BstockMarketService, bstock_trade::BstockTradeService,
    wallet_portfolio::WalletPortfolioService,
};

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

pub fn trade_router(service: Arc<BstockTradeService>) -> Router {
    Router::new()
        .route("/v1/bstock-trades/quotes", post(handlers::trades::quotes))
        .route(
            "/v1/bstock-trades/approval",
            post(handlers::trades::prepare_approval),
        )
        .route(
            "/v1/bstock-trades/prepare",
            post(handlers::trades::prepare_trade),
        )
        .route(
            "/v1/bstock-trades/submit",
            post(handlers::trades::submit_trade),
        )
        .route(
            "/v1/bstock-trades/orders/{order_id}",
            get(handlers::trades::trade_status),
        )
        .with_state(service)
}
