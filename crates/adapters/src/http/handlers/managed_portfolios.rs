use std::sync::Arc;

use axum::extract::{Query, State};
use stockfront_core::managed_portfolios::{ManagedPortfolioService, ManagedPortfolioTemplatePage};

use super::super::{
    primitives::{ApiResult, Response},
    request::ListManagedPortfolioTemplatesRequest,
};

pub(in crate::http) async fn templates(
    State(service): State<Arc<ManagedPortfolioService>>,
    Query(request): Query<ListManagedPortfolioTemplatesRequest>,
) -> ApiResult<ManagedPortfolioTemplatePage> {
    let query = request.try_into()?;
    Ok(Response::ok(service.templates(&query).await?))
}
