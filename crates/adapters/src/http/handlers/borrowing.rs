use std::sync::Arc;

use alloy_primitives::B256;
use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
};
use stockfront_core::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingMarket, BorrowingMarketQuery, BorrowingService,
        BorrowingTransactionStatus, PrepareBorrowRequest, PrepareCollateralRequest,
        PrepareRepaymentRequest, PrepareSupplyRequest, PrepareWithdrawalRequest,
        PreparedBorrowingAction,
    },
    bstock_market::Address,
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, ErrorResponse, List, Response},
    request::BorrowingMarketsRequest,
};
use super::parse_wallet;

pub(in crate::http) async fn markets(
    State(service): State<Arc<BorrowingService>>,
    Query(request): Query<BorrowingMarketsRequest>,
) -> ApiResult<List<BorrowingMarket>> {
    let query = BorrowingMarketQuery {
        collateral_token: parse_optional_address(request.collateral_token)?,
        debt_token: parse_optional_address(request.debt_token)?,
    };
    let markets = service.markets(query).await?;
    Ok(Response::ok(List::all(markets)))
}

#[utoipa::path(
    get, path = "/v1/wallets/{address}/loans",
    params(("address" = String, Path)),
    responses(
        (status = 200, body = Response<BorrowingAccount>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn account(
    State(service): State<Arc<BorrowingService>>,
    Path(address): Path<String>,
) -> ApiResult<BorrowingAccount> {
    let account = service.account(parse_wallet(&address)?).await?;
    Ok(Response::ok(account))
}

pub(in crate::http) async fn prepare_supply(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareSupplyRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let supply = service.prepare_supply(&request).await?;
    Ok(Response::ok(supply))
}

pub(in crate::http) async fn prepare_collateral(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareCollateralRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let collateral = service.prepare_collateral(&request).await?;
    Ok(Response::ok(collateral))
}

pub(in crate::http) async fn prepare_borrow(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareBorrowRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let borrow = service.prepare_borrow(&request).await?;
    Ok(Response::ok(borrow))
}

pub(in crate::http) async fn prepare_repayment_approval(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareRepaymentRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let approval = service.prepare_repayment_approval(&request).await?;
    Ok(Response::ok(approval))
}

pub(in crate::http) async fn prepare_repayment(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareRepaymentRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let repayment = service.prepare_repayment(&request).await?;
    Ok(Response::ok(repayment))
}

pub(in crate::http) async fn prepare_withdrawal(
    State(service): State<Arc<BorrowingService>>,
    request: Result<Json<PrepareWithdrawalRequest>, JsonRejection>,
) -> ApiResult<PreparedBorrowingAction> {
    let Json(request) = request?;
    let withdrawal = service.prepare_withdrawal(&request).await?;
    Ok(Response::ok(withdrawal))
}

pub(in crate::http) async fn transaction_status(
    State(service): State<Arc<BorrowingService>>,
    Path(transaction_hash): Path<String>,
) -> ApiResult<BorrowingTransactionStatus> {
    let transaction_hash = transaction_hash
        .parse::<B256>()
        .map_err(|_| ApiError::INVALID_TRANSACTION_HASH)?;
    let status = service.transaction_status(transaction_hash).await?;
    Ok(Response::ok(status))
}

fn parse_optional_address(value: Option<String>) -> Result<Option<Address>, ApiError> {
    value
        .map(|value| value.parse().map_err(|_| ApiError::INVALID_TOKEN_ADDRESS))
        .transpose()
}
