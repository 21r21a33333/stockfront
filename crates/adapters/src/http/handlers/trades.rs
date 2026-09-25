use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
};
use stockfront_core::bstock_trade::{
    ApprovalRequest, BstockTradeService, PrepareTradeRequest, PreparedApproval, PreparedTrade,
    SubmitOrderRequest, SubmittedOrder, TradeOrderStatus, TradeQuote, TradeQuoteRequest,
};

use super::super::primitives::{ApiResult, List, Response};

pub(in crate::http) async fn quotes(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<TradeQuoteRequest>, JsonRejection>,
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

pub(in crate::http) async fn submit_order(
    State(service): State<Arc<BstockTradeService>>,
    request: Result<Json<SubmitOrderRequest>, JsonRejection>,
) -> ApiResult<SubmittedOrder> {
    let Json(request) = request?;
    let order = service.submit_order(&request).await?;
    Ok(Response::ok(order))
}

pub(in crate::http) async fn order_status(
    State(service): State<Arc<BstockTradeService>>,
    Path(order_id): Path<String>,
) -> ApiResult<TradeOrderStatus> {
    let status = service.order_status(&order_id).await?;
    Ok(Response::ok(status))
}
