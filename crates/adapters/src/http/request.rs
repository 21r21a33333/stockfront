use serde::Deserialize;
use stockfront_core::bstock_market::{ListBstocksQuery, QueryError};

#[derive(Debug, Deserialize)]
pub(super) struct ListBstocksRequest {
    query: Option<String>,
    limit: Option<usize>,
}

impl TryFrom<ListBstocksRequest> for ListBstocksQuery {
    type Error = QueryError;

    fn try_from(value: ListBstocksRequest) -> Result<Self, Self::Error> {
        Self::new(value.query, value.limit)
    }
}
