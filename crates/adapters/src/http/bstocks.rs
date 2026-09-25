use std::sync::Arc;

use axum::extract::{Path, Query, State};
use stockfront_core::bstock_market::{
    Address, BstockDetails, BstockId, BstockMarketService, BstockSummary,
};

use super::{
    error::ApiError,
    primitives::{ApiResult, List, Response},
    request::ListBstocksRequest,
};

pub(super) async fn list_bstocks(
    State(service): State<Arc<BstockMarketService>>,
    Query(request): Query<ListBstocksRequest>,
) -> ApiResult<List<BstockSummary>> {
    let query = request.try_into()?;
    let assets = service.list(&query).await?;
    Ok(Response::ok(List::all(assets)))
}

pub(super) async fn get_bstock(
    State(service): State<Arc<BstockMarketService>>,
    Path((chain_id, contract)): Path<(u64, String)>,
) -> ApiResult<BstockDetails> {
    let contract = contract
        .parse::<Address>()
        .map_err(|_| ApiError::InvalidAddress)?;
    let details = service
        .details(BstockId { chain_id, contract })
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Response::ok(details))
}
