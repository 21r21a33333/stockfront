use std::sync::Arc;

use axum::extract::{Query, State};
use stockfront_core::baskets::{BasketService, BasketSummary};

use super::super::{
    primitives::{ApiResult, List, Response},
    request::ListBasketsRequest,
};

pub(in crate::http) async fn list_baskets(
    State(service): State<Arc<BasketService>>,
    Query(request): Query<ListBasketsRequest>,
) -> ApiResult<List<BasketSummary>> {
    let query = request.try_into()?;
    let baskets = service.list(&query).await?;
    Ok(Response::ok(List::all(baskets)))
}
