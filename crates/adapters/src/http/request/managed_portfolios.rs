use serde::Deserialize;
use stockfront_core::managed_portfolios::{
    ListManagedPortfolioTemplatesQuery, ListManagedPortfolioTemplatesQueryError,
    ListManagedPortfoliosQuery, ListManagedPortfoliosQueryError, ManagedPortfolioCollection,
    ManagedPortfolioSort,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::http) struct ListManagedPortfolioTemplatesRequest {
    collection: Option<ManagedPortfolioCollection>,
    sort: Option<ManagedPortfolioSort>,
    limit: Option<usize>,
    cursor: Option<String>,
}

impl TryFrom<ListManagedPortfolioTemplatesRequest> for ListManagedPortfolioTemplatesQuery {
    type Error = ListManagedPortfolioTemplatesQueryError;

    fn try_from(value: ListManagedPortfolioTemplatesRequest) -> Result<Self, Self::Error> {
        Self::new(value.collection, value.sort, value.limit, value.cursor)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::http) struct ListManagedPortfoliosRequest {
    limit: Option<usize>,
    cursor: Option<String>,
}

impl TryFrom<ListManagedPortfoliosRequest> for ListManagedPortfoliosQuery {
    type Error = ListManagedPortfoliosQueryError;

    fn try_from(value: ListManagedPortfoliosRequest) -> Result<Self, Self::Error> {
        Self::new(value.limit, value.cursor)
    }
}
