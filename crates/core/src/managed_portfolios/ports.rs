use async_trait::async_trait;
use thiserror::Error;

use super::{ListManagedPortfolioTemplatesQuery, ManagedPortfolioTemplatePage};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ManagedPortfolioProviderError {
    #[error("the managed portfolio provider is temporarily unavailable")]
    Unavailable,
    #[error("the managed portfolio provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait ManagedPortfolioReader: Send + Sync {
    async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ManagedPortfolioTemplatePage, ManagedPortfolioProviderError>;
}
