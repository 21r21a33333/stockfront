//! The recipe builder's API contract, generated from Rust handlers and models.

use axum::Json;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Stockfront Recipe API", version = "0.1.0"),
    components(schemas(
        utoipa::TupleUnit,
        stockfront_core::recipes::RecipeCatalog,
        stockfront_core::recipes::RecipeValidationReport,
        stockfront_core::recipes::RecipePreview,
        stockfront_core::recipes::PreparedRecipe,
        stockfront_core::bstock_trade::SubmittedOrder
    )),
    paths(
        super::handlers::recipes::primitives,
        super::handlers::recipes::validate,
        super::handlers::recipes::preview,
        super::handlers::recipes::prepare,
        super::handlers::trades::submit_order,
    )
)]
pub struct ApiDoc;

pub(super) async fn document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
