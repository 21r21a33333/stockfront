use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State, rejection::JsonRejection, rejection::QueryRejection},
};
use stockfront_core::{
    bstock_market::Address,
    carry::{
        CarryOpportunity, CarryOpportunityQuery, CarryService, PrepareCarryDepositRequest,
        PreparedCarryDeposit,
    },
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, List, Response},
    request::CarryOpportunitiesRequest,
};

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

fn parse_address(value: &str) -> Result<Address, ApiError> {
    value.parse().map_err(|_| ApiError::INVALID_TOKEN_ADDRESS)
}
