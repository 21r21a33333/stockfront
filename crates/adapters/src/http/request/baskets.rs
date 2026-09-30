use serde::Deserialize;
use stockfront_core::baskets::{ListBasketsQuery, ListBasketsQueryError};

#[derive(Debug, Deserialize)]
pub(in crate::http) struct ListBasketsRequest {
    query: Option<String>,
    category: Option<String>,
    limit: Option<usize>,
}

impl TryFrom<ListBasketsRequest> for ListBasketsQuery {
    type Error = ListBasketsQueryError;

    fn try_from(value: ListBasketsRequest) -> Result<Self, Self::Error> {
        Self::new(value.query, value.category, value.limit)
    }
}
