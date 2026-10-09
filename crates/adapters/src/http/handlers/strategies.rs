use std::sync::Arc;

use axum::{Json, extract::State, extract::rejection::JsonRejection};
use stockfront_core::strategy_composer::{
    ComposeStrategyRequest, ComposedStrategy, StrategyComposerService,
};

use super::super::primitives::{ApiResult, ErrorResponse, Response};

#[utoipa::path(
    post, path = "/v1/strategies/compose",
    request_body = ComposeStrategyRequest,
    responses(
        (status = 200, body = Response<ComposedStrategy>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn compose(
    State(service): State<Arc<StrategyComposerService>>,
    request: Result<Json<ComposeStrategyRequest>, JsonRejection>,
) -> ApiResult<ComposedStrategy> {
    let Json(request) = request?;
    let composed = service.compose(&request).await?;
    Ok(Response::ok(composed))
}
