use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
};
use stockfront_core::managed_portfolios::{
    EnrollManagedPortfolioRequest, ManagedPortfolioEnrollment, ManagedPortfolioOperation,
    ManagedPortfolioPage, ManagedPortfolioPositions, ManagedPortfolioService,
    ManagedPortfolioTemplatePage, PrepareManagedPortfolioDepositRequest,
    PrepareManagedPortfolioEnrollmentRequest, PrepareManagedPortfolioWithdrawalRequest,
    PreparedManagedPortfolioDeposit, PreparedManagedPortfolioEnrollment,
    PreparedManagedPortfolioWithdrawal, SubmitManagedPortfolioWithdrawalRequest,
    SubmittedManagedPortfolioWithdrawal,
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

pub(in crate::http) async fn prepare_deposit(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path(portfolio_id): Path<String>,
    request: Result<Json<PrepareManagedPortfolioDepositRequest>, JsonRejection>,
) -> ApiResult<PreparedManagedPortfolioDeposit> {
    let Json(request) = request?;
    Ok(Response::ok(
        service.prepare_deposit(&portfolio_id, &request).await?,
    ))
}

pub(in crate::http) async fn prepare_withdrawal(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path(portfolio_id): Path<String>,
    request: Result<Json<PrepareManagedPortfolioWithdrawalRequest>, JsonRejection>,
) -> ApiResult<PreparedManagedPortfolioWithdrawal> {
    let Json(request) = request?;
    Ok(Response::ok(
        service.prepare_withdrawal(&portfolio_id, &request).await?,
    ))
}

pub(in crate::http) async fn submit_withdrawal(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path(portfolio_id): Path<String>,
    request: Result<Json<SubmitManagedPortfolioWithdrawalRequest>, JsonRejection>,
) -> ApiResult<SubmittedManagedPortfolioWithdrawal> {
    let Json(request) = request?;
    Ok(Response::ok(
        service.submit_withdrawal(&portfolio_id, &request).await?,
    ))
}

pub(in crate::http) async fn operation(
    State(service): State<Arc<ManagedPortfolioService>>,
    Path((portfolio_id, operation_id)): Path<(String, String)>,
) -> ApiResult<ManagedPortfolioOperation> {
    Ok(Response::ok(
        service.operation(&portfolio_id, &operation_id).await?,
    ))
}
