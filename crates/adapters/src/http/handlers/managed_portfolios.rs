use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
};
use stockfront_core::managed_portfolios::{
    EnrollManagedPortfolioRequest, ManagedPortfolioEnrollment, ManagedPortfolioPage,
    ManagedPortfolioPositions, ManagedPortfolioService, ManagedPortfolioTemplatePage,
    PrepareManagedPortfolioEnrollmentRequest, PreparedManagedPortfolioEnrollment,
};

use super::super::{
    primitives::{ApiResult, Response},
    request::{ListManagedPortfolioTemplatesRequest, ListManagedPortfoliosRequest},
};
use super::parse_wallet;

pub(in crate::http) async fn templates(
    State(service): State<Arc<ManagedPortfolioService>>,
    Query(request): Query<ListManagedPortfolioTemplatesRequest>,
) -> ApiResult<ManagedPortfolioTemplatePage> {
    let query = request.try_into()?;
    Ok(Response::ok(service.templates(&query).await?))
}

pub(in crate::http) async fn prepare_enrollment(
    State(service): State<Arc<ManagedPortfolioService>>,
    request: Result<Json<PrepareManagedPortfolioEnrollmentRequest>, JsonRejection>,
) -> ApiResult<PreparedManagedPortfolioEnrollment> {
    let Json(request) = request?;
    Ok(Response::ok(service.prepare_enrollment(&request).await?))
}

pub(in crate::http) async fn enroll(
    State(service): State<Arc<ManagedPortfolioService>>,
    request: Result<Json<EnrollManagedPortfolioRequest>, JsonRejection>,
) -> ApiResult<ManagedPortfolioEnrollment> {
    let Json(request) = request?;
    Ok(Response::ok(service.enroll(&request).await?))
}

pub(in crate::http) async fn portfolios(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path(address): Path<String>,
    Query(request): Query<ListManagedPortfoliosRequest>,
) -> ApiResult<ManagedPortfolioPage> {
    let wallet = parse_wallet(&address)?;
    let query = request.try_into()?;
    Ok(Response::ok(service.portfolios(wallet, &query).await?))
}

pub(in crate::http) async fn positions(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path(portfolio_id): Path<String>,
) -> ApiResult<ManagedPortfolioPositions> {
    Ok(Response::ok(service.positions(&portfolio_id).await?))
}
