use std::sync::Arc;

use axum::extract::{Path, State};
use stockfront_core::wallet_portfolio::{WalletPortfolio, WalletPortfolioService};

use super::super::primitives::{ApiResult, Response};
use super::parse_wallet;

pub(in crate::http) async fn portfolio(
    State(service): State<Arc<WalletPortfolioService>>,
    Path(address): Path<String>,
) -> ApiResult<WalletPortfolio> {
    let portfolio = service.portfolio(parse_wallet(&address)?).await?;
    Ok(Response::ok(portfolio))
}
