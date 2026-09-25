use std::sync::Arc;

use axum::extract::{Path, State};
use stockfront_core::{
    bstock_market::Address,
    wallet_portfolio::{WalletAddress, WalletPortfolio, WalletPortfolioService},
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, Response},
};

pub(in crate::http) async fn portfolio(
    State(service): State<Arc<WalletPortfolioService>>,
    Path(address): Path<String>,
) -> ApiResult<WalletPortfolio> {
    let address = address
        .parse::<Address>()
        .map_err(|_| ApiError::InvalidWalletAddress)?;
    let wallet = WalletAddress::try_from(address)?;
    let portfolio = service.portfolio(wallet).await?;
    Ok(Response::ok(portfolio))
}
