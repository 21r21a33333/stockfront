//! Inbound HTTP adapter for the bStock market slice.

mod bstocks;
mod error;
mod primitives;
mod request;

use std::sync::Arc;

use axum::{Router, routing::get};
use stockfront_core::bstock_market::BstockMarketService;

pub fn bstock_router(service: Arc<BstockMarketService>) -> Router {
    Router::new()
        .route("/v1/bstocks", get(bstocks::list_bstocks))
        .route(
            "/v1/bstocks/{chain_id}/{contract}",
            get(bstocks::get_bstock),
        )
        .with_state(service)
}
