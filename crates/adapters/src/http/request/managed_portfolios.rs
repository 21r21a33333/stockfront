use serde::Deserialize;
use stockfront_core::managed_portfolios::{
    ListManagedPortfolioTemplatesQuery, ListManagedPortfolioTemplatesQueryError,
    ManagedPortfolioCollection, ManagedPortfolioSort,
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
