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
        stockfront_core::bstock_borrowing::BorrowingAccount,
        stockfront_core::bstock_trade::SubmittedOrder,
        stockfront_core::strategy_composer::ComposedStrategy
    )),
    paths(
        super::handlers::recipes::primitives,
        super::handlers::recipes::validate,
        super::handlers::recipes::preview,
        super::handlers::recipes::prepare,
        super::handlers::strategies::compose,
        super::handlers::borrowing::account,
        super::handlers::carry::opportunities,
        super::handlers::carry::positions,
        super::handlers::trades::submit_order,
        super::handlers::trades::order_status,
        super::handlers::trades::transaction_status,
    )
)]
pub struct ApiDoc;

pub(super) async fn document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use serde_json::to_value;
    use utoipa::OpenApi;

    use super::ApiDoc;

    #[test]
    fn loans_path_exports_the_borrowing_account_contract() {
        let document = to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
        let response = &document["paths"]["/v1/wallets/{address}/loans"]["get"]["responses"]["200"];

        assert!(!response.is_null());
        assert!(response.to_string().contains("BorrowingAccount"));
        assert!(document["components"]["schemas"]["BorrowingAccount"].is_object());
    }

    #[test]
    fn recipe_contract_exports_repayment_types() {
        let document = to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
        let schemas = &document["components"]["schemas"];

        assert!(schemas["RepaymentCapability"].is_object());
        assert!(schemas["RecipeRepayPolicy"].is_object());
        assert!(schemas["RecipePrimitive"].to_string().contains("REPAY"));
        assert!(schemas["PreparedRecipeStep"].to_string().contains("REPAY"));
    }
}
