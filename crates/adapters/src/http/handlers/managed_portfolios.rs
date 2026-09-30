use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State, rejection::JsonRejection},
};
use stockfront_core::managed_portfolios::{
    EnrollManagedPortfolioRequest, ManagedPortfolioEnrollment, ManagedPortfolioService,
    ManagedPortfolioTemplatePage, PrepareManagedPortfolioEnrollmentRequest,
    PreparedManagedPortfolioEnrollment,
};

use super::super::{
    primitives::{ApiResult, Response},
    request::ListManagedPortfolioTemplatesRequest,
};

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
