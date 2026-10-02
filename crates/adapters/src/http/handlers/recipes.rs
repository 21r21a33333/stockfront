use std::sync::Arc;

use axum::{Json, extract::State, extract::rejection::JsonRejection};
use stockfront_core::recipes::{
    PrepareRecipeRequest, PreparedRecipe, RecipeCatalog, RecipeDefinition, RecipePreview,
    RecipePreviewRequest, RecipeService, RecipeValidationReport,
};

use super::super::primitives::{ApiResult, Response};

pub(in crate::http) async fn primitives(
    State(service): State<Arc<RecipeService>>,
) -> ApiResult<RecipeCatalog> {
    let catalog = service.catalog().await?;
    Ok(Response::ok(catalog))
}

pub(in crate::http) async fn validate(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<RecipeDefinition>, JsonRejection>,
) -> ApiResult<RecipeValidationReport> {
    let Json(recipe) = request?;
    Ok(Response::ok(service.validate(&recipe)))
}

pub(in crate::http) async fn preview(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<RecipePreviewRequest>, JsonRejection>,
) -> ApiResult<RecipePreview> {
    let Json(request) = request?;
    let preview = service.preview(&request).await?;
    Ok(Response::ok(preview))
}

pub(in crate::http) async fn prepare(
    State(service): State<Arc<RecipeService>>,
    request: Result<Json<PrepareRecipeRequest>, JsonRejection>,
) -> ApiResult<PreparedRecipe> {
    let Json(request) = request?;
    let prepared = service.prepare(&request).await?;
    Ok(Response::ok(prepared))
}
