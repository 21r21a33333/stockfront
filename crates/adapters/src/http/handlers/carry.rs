use std::sync::Arc;

use axum::extract::{Query, State, rejection::QueryRejection};
use stockfront_core::{
    bstock_market::Address,
    carry::{CarryOpportunity, CarryOpportunityQuery, CarryOpportunityService},
};

use super::super::{
    error::ApiError,
    primitives::{ApiResult, List, Response},
    request::CarryOpportunitiesRequest,
};

pub(in crate::http) async fn opportunities(
    State(service): State<Arc<CarryOpportunityService>>,
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

fn parse_address(value: &str) -> Result<Address, ApiError> {
    value.parse().map_err(|_| ApiError::INVALID_TOKEN_ADDRESS)
}
