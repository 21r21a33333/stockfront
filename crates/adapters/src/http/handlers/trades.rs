use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
};
use stockfront_core::bstock_trade::{
    ApprovalRequest, BstockTradeService, PrepareTradeRequest, PreparedApproval, PreparedTrade,
    SubmitTradeRequest, SubmittedTrade, TradeQuote, TradeRequest, TradeStatus,
};

use super::super::primitives::{ApiResult, List, Response};

pub(in crate::http) async fn quotes(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<TradeRequest>, JsonRejection>,
) -> ApiResult<List<TradeQuote>> {
    let Json(request) = request?;
    let quotes = service.quotes(&request).await?;
    Ok(Response::ok(List::all(quotes)))
}

pub(in crate::http) async fn prepare_approval(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<ApprovalRequest>, JsonRejection>,
) -> ApiResult<PreparedApproval> {
    let Json(request) = request?;
    let approval = service.prepare_approval(&request).await?;
    Ok(Response::ok(approval))
}

pub(in crate::http) async fn prepare_trade(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<PrepareTradeRequest>, JsonRejection>,
) -> ApiResult<PreparedTrade> {
    let Json(request) = request?;
    let trade = service.prepare_trade(&request).await?;
    Ok(Response::ok(trade))
}

pub(in crate::http) async fn submit_trade(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<SubmitTradeRequest>, JsonRejection>,
) -> ApiResult<SubmittedTrade> {
    let Json(request) = request?;
    let trade = service.submit_trade(&request).await?;
    Ok(Response::ok(trade))
}

pub(in crate::http) async fn trade_status(
    State(service): State<Arc<BstockTradeService>>,
    Path(order_id): Path<String>,
) -> ApiResult<TradeStatus> {
    let status = service.trade_status(&order_id).await?;
    Ok(Response::ok(status))
}
