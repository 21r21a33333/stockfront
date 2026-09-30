use async_trait::async_trait;
use thiserror::Error;

use super::BasketSummary;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BasketProviderError {
    #[error("the basket provider is temporarily unavailable")]
    Unavailable,
    #[error("the basket provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait BasketCatalog: Send + Sync {
    async fn list_baskets(&self) -> Result<Vec<BasketSummary>, BasketProviderError>;
}
