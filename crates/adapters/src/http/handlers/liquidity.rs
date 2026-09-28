use std::sync::Arc;

use axum::extract::{Query, State, rejection::QueryRejection};
use stockfront_core::liquidity::{LiquidityOpportunity, LiquidityService};

use super::{
    super::{
        primitives::{ApiResult, List, Response},
        request::LiquidityOpportunitiesRequest,
    },
    parse_token_address,
};

pub(in crate::http) async fn opportunities(
    State(service): State<Arc<LiquidityService>>,
    request: Result<Query<LiquidityOpportunitiesRequest>, QueryRejection>,
) -> ApiResult<List<LiquidityOpportunity>> {
    let Query(request) = request?;
    let opportunities = service
        .opportunities(parse_token_address(&request.bstock_token)?)
        .await?;
    Ok(Response::ok(List::all(opportunities)))
}
