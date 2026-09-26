use std::sync::Arc;

use axum::extract::{Path, Query, State};
use stockfront_core::{
    bstock_borrowing::{BorrowingAccount, BorrowingMarket, BorrowingMarketQuery, BorrowingService},
    bstock_market::Address,
    wallet_portfolio::WalletAddress,
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, List, Response},
    request::BorrowingMarketsRequest,
};

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

pub(in crate::http) async fn account(
    State(service): State<Arc<BorrowingService>>,
    Path(address): Path<String>,
) -> ApiResult<BorrowingAccount> {
    let address = address
        .parse::<Address>()
        .map_err(|_| ApiError::INVALID_WALLET_ADDRESS)?;
    let account = service.account(WalletAddress::try_from(address)?).await?;
    Ok(Response::ok(account))
}

fn parse_optional_address(value: Option<String>) -> Result<Option<Address>, ApiError> {
    value
        .map(|value| value.parse().map_err(|_| ApiError::INVALID_TOKEN_ADDRESS))
        .transpose()
}
