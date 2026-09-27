//! Inbound HTTP adapter for Stockfront's public API.

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
    bstock_borrowing::BorrowingService, bstock_market::BstockMarketService,
    bstock_trade::BstockTradeService, carry::CarryService,
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

pub fn borrowing_router(service: Arc<BorrowingService>) -> Router {
    Router::new()
        .route("/v1/borrowing/markets", get(handlers::borrowing::markets))
        .route(
            "/v1/wallets/{address}/loans",
            get(handlers::borrowing::account),
        )
        .route(
            "/v1/borrowing/supplies/prepare",
            post(handlers::borrowing::prepare_supply),
        )
        .route(
            "/v1/borrowing/collateral/prepare",
            post(handlers::borrowing::prepare_collateral),
        )
        .route(
            "/v1/borrowing/borrows/prepare",
            post(handlers::borrowing::prepare_borrow),
        )
        .route(
            "/v1/borrowing/repayments/approval",
            post(handlers::borrowing::prepare_repayment_approval),
        )
        .route(
            "/v1/borrowing/repayments/prepare",
            post(handlers::borrowing::prepare_repayment),
        )
        .route(
            "/v1/borrowing/withdrawals/prepare",
            post(handlers::borrowing::prepare_withdrawal),
        )
        .route(
            "/v1/borrowing/transactions/{transaction_hash}",
            get(handlers::borrowing::transaction_status),
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
            "/v1/bstock-trades/orders",
            post(handlers::trades::submit_order),
        )
        .route(
            "/v1/bstock-trades/orders/{order_id}",
            get(handlers::trades::order_status),
        )
        .route(
            "/v1/bstock-trades/transactions/{transaction_hash}",
            get(handlers::trades::transaction_status),
        )
        .with_state(service)
}

pub fn carry_router(service: Arc<CarryService>) -> Router {
    Router::new()
        .route(
            "/v1/carry/opportunities",
            get(handlers::carry::opportunities),
        )
        .route(
            "/v1/carry/deposits/prepare",
            post(handlers::carry::prepare_deposit),
        )
        .route(
            "/v1/wallets/{address}/carry-positions",
            get(handlers::carry::positions),
        )
        .with_state(service)
}
