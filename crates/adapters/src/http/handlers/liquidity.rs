use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection, rejection::QueryRejection},
};
use stockfront_core::liquidity::{
    LiquidityAccount, LiquidityOpportunity, LiquidityService, PrepareLiquidityClaimRequest,
    PrepareLiquidityEntryRequest, PrepareLiquidityWithdrawalRequest, PreparedLiquidityAction,
    PreparedLiquidityEntry,
};

use super::{
    super::{
        primitives::{ApiResult, List, Response},
        request::LiquidityOpportunitiesRequest,
    },
    parse_token_address, parse_wallet,
};

pub(in crate::http) async fn opportunities(
    State(service): State<Arc<LiquidityService>>,
    request: Result<Query<LiquidityOpportunitiesRequest>, QueryRejection>,
) -> ApiResult<List<LiquidityOpportunity>> {
    let Query(request) = request?;
    let opportunities = service
        .opportunities(parse_token_address(&request.bstock_token)?)
        .await?;
    Ok(Response::ok(List::all(opportunities)))
}

pub(in crate::http) async fn positions(
    State(service): State<Arc<LiquidityService>>,
    Path(address): Path<String>,
) -> ApiResult<LiquidityAccount> {
    let positions = service.positions(parse_wallet(&address)?).await?;
    Ok(Response::ok(positions))
}

pub(in crate::http) async fn prepare_entry(
    State(service): State<Arc<LiquidityService>>,
    request: Result<Json<PrepareLiquidityEntryRequest>, JsonRejection>,
) -> ApiResult<PreparedLiquidityEntry> {
    let Json(request) = request?;
    let entry = service.prepare_entry(&request).await?;
    Ok(Response::ok(entry))
}

pub(in crate::http) async fn prepare_claim(
    State(service): State<Arc<LiquidityService>>,
    request: Result<Json<PrepareLiquidityClaimRequest>, JsonRejection>,
) -> ApiResult<PreparedLiquidityAction> {
    let Json(request) = request?;
    let claim = service.prepare_claim(&request).await?;
    Ok(Response::ok(claim))
}

pub(in crate::http) async fn prepare_withdrawal(
    State(service): State<Arc<LiquidityService>>,
    request: Result<Json<PrepareLiquidityWithdrawalRequest>, JsonRejection>,
) -> ApiResult<PreparedLiquidityAction> {
    let Json(request) = request?;
    let withdrawal = service.prepare_withdrawal(&request).await?;
    Ok(Response::ok(withdrawal))
}
