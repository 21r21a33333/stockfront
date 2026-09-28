use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection, rejection::QueryRejection},
};
use stockfront_core::{
    bstock_market::Address,
    carry::{
        CarryAccount, CarryActionPlan, CarryOpportunity, CarryOpportunityQuery, CarryService,
        PrepareCarryDepositRequest, PrepareCarryRedemptionClaimRequest,
        PrepareCarryWithdrawalRequest, PreparedCarryDeposit,
    },
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, List, Response},
    request::CarryOpportunitiesRequest,
};
use super::parse_wallet;

pub(in crate::http) async fn opportunities(
    State(service): State<Arc<CarryService>>,
    request: Result<Query<CarryOpportunitiesRequest>, QueryRejection>,
) -> ApiResult<List<CarryOpportunity>> {
    let Query(request) = request?;
    let opportunities = service
        .opportunities(CarryOpportunityQuery {
            collateral_token: parse_address(&request.collateral_token)?,
            debt_token: parse_address(&request.debt_token)?,
        })
        .await?;
    Ok(Response::ok(List::all(opportunities)))
}

pub(in crate::http) async fn prepare_deposit(
    State(service): State<Arc<CarryService>>,
    request: Result<Json<PrepareCarryDepositRequest>, JsonRejection>,
) -> ApiResult<PreparedCarryDeposit> {
    let Json(request) = request?;
    let deposit = service.prepare_deposit(&request).await?;
    Ok(Response::ok(deposit))
}

pub(in crate::http) async fn prepare_withdrawal(
    State(service): State<Arc<CarryService>>,
    request: Result<Json<PrepareCarryWithdrawalRequest>, JsonRejection>,
) -> ApiResult<CarryActionPlan> {
    let Json(request) = request?;
    let withdrawal = service.prepare_withdrawal(&request).await?;
    Ok(Response::ok(withdrawal))
}

pub(in crate::http) async fn prepare_redemption_claim(
    State(service): State<Arc<CarryService>>,
    request: Result<Json<PrepareCarryRedemptionClaimRequest>, JsonRejection>,
) -> ApiResult<CarryActionPlan> {
    let Json(request) = request?;
    let claim = service.prepare_redemption_claim(&request).await?;
    Ok(Response::ok(claim))
}

pub(in crate::http) async fn positions(
    State(service): State<Arc<CarryService>>,
    Path(address): Path<String>,
) -> ApiResult<CarryAccount> {
    let positions = service.positions(parse_wallet(&address)?).await?;
    Ok(Response::ok(positions))
}

fn parse_address(value: &str) -> Result<Address, ApiError> {
    value.parse().map_err(|_| ApiError::INVALID_TOKEN_ADDRESS)
}
