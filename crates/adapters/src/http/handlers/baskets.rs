use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
};
use stockfront_core::{
    baskets::{BasketAccount, BasketService, BasketSummary},
    bstock_trade::{
        ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitOrderRequest,
        SubmittedOrder, TradeOrderStatus, TradeQuote, TradeQuoteRequest,
    },
};

use super::super::{
    primitives::{ApiResult, List, Response},
    request::ListBasketsRequest,
};
use super::parse_wallet;

pub(in crate::http) async fn list_baskets(
    State(service): State<Arc<BasketService>>,
    Query(request): Query<ListBasketsRequest>,
) -> ApiResult<List<BasketSummary>> {
    let query = request.try_into()?;
    let baskets = service.list(&query).await?;
    Ok(Response::ok(List::all(baskets)))
}

pub(in crate::http) async fn positions(
    State(service): State<Arc<BasketService>>,
    Path(address): Path<String>,
) -> ApiResult<BasketAccount> {
    Ok(Response::ok(
        service.positions(parse_wallet(&address)?).await?,
    ))
}

pub(in crate::http) async fn quotes(
    State(service): State<Arc<BasketService>>,
    request: Result<Json<TradeQuoteRequest>, JsonRejection>,
) -> ApiResult<List<TradeQuote>> {
    let Json(request) = request?;
    Ok(Response::ok(List::all(service.quotes(&request).await?)))
}

pub(in crate::http) async fn prepare_approval(
    State(service): State<Arc<BasketService>>,
    request: Result<Json<ApprovalRequest>, JsonRejection>,
) -> ApiResult<PreparedApproval> {
    let Json(request) = request?;
    Ok(Response::ok(service.prepare_approval(&request).await?))
}

pub(in crate::http) async fn prepare_trade(
    State(service): State<Arc<BasketService>>,
    request: Result<Json<PrepareTradeRequest>, JsonRejection>,
) -> ApiResult<PreparedTrade> {
    let Json(request) = request?;
    Ok(Response::ok(service.prepare_trade(&request).await?))
}

pub(in crate::http) async fn submit_order(
    State(service): State<Arc<BasketService>>,
    request: Result<Json<SubmitOrderRequest>, JsonRejection>,
) -> ApiResult<SubmittedOrder> {
    let Json(request) = request?;
    Ok(Response::ok(service.submit_order(&request).await?))
}

pub(in crate::http) async fn order_status(
    State(service): State<Arc<BasketService>>,
    Path(order_id): Path<String>,
) -> ApiResult<TradeOrderStatus> {
    Ok(Response::ok(service.order_status(&order_id).await?))
}
