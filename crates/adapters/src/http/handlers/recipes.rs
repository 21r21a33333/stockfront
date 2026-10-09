use std::sync::Arc;

use axum::{Json, extract::State, extract::rejection::JsonRejection};
use stockfront_core::recipes::{
    PrepareRecipeRequest, PreparedRecipe, RecipeCatalog, RecipeDefinition, RecipePreview,
    RecipePreviewRequest, RecipeService, RecipeValidationReport,
};

use super::super::primitives::{ApiResult, ErrorResponse, Response};

#[utoipa::path(
    get, path = "/v1/recipe-primitives",
    responses(
        (status = 200, body = Response<RecipeCatalog>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn primitives(
    State(service): State<Arc<RecipeService>>,
) -> ApiResult<RecipeCatalog> {
    let catalog = service.catalog().await?;
    Ok(Response::ok(catalog))
}

#[utoipa::path(
    post, path = "/v1/recipes/validate",
    request_body = RecipeDefinition,
    responses(
        (status = 200, body = Response<RecipeValidationReport>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn validate(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<RecipeDefinition>, JsonRejection>,
) -> ApiResult<RecipeValidationReport> {
    let Json(recipe) = request?;
    Ok(Response::ok(service.validate(&recipe)))
}

#[utoipa::path(
    post, path = "/v1/recipes/preview",
    request_body = RecipePreviewRequest,
    responses(
        (status = 200, body = Response<RecipePreview>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn preview(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<RecipePreviewRequest>, JsonRejection>,
) -> ApiResult<RecipePreview> {
    let Json(request) = request?;
    let preview = service.preview(&request).await?;
    Ok(Response::ok(preview))
}

#[utoipa::path(
    post, path = "/v1/recipes/prepare",
    request_body = PrepareRecipeRequest,
    responses(
        (status = 200, body = Response<PreparedRecipe>),
        (status = "default", body = ErrorResponse)
    )
)]
pub(in crate::http) async fn prepare(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<PrepareRecipeRequest>, JsonRejection>,
) -> ApiResult<PreparedRecipe> {
    let Json(request) = request?;
    let prepared = service.prepare(&request).await?;
    Ok(Response::ok(prepared))
}
