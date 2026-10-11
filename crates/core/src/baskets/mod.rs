mod model;
mod ports;
mod service;

pub use model::{
    BasketAccount, BasketAssetAllocation, BasketId, BasketKind, BasketPosition, BasketSummary,
    ListBasketsQuery, ListBasketsQueryError,
};
pub use ports::{BasketProviderError, BasketReader};
pub use service::BasketService;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::{BasketAccount, BasketProviderError, BasketReader, BasketSummary};
    use crate::wallet_portfolio::WalletAddress;

    /// A basket venue with nothing listed. Tests that exercise the bStock side
    /// need one, and an empty catalog is what sends a trade there rather than here.
    pub(crate) struct NoBaskets;

    #[async_trait]
    impl BasketReader for NoBaskets {
        async fn list_baskets(&self) -> Result<Vec<BasketSummary>, BasketProviderError> {
            Ok(Vec::new())
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BasketAccount, BasketProviderError> {
            Err(BasketProviderError::Unavailable)
        }
    }

    #[must_use]
    pub(crate) fn no_baskets() -> Arc<dyn BasketReader> {
        Arc::new(NoBaskets)
    }
}
