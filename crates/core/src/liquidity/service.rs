use std::sync::Arc;

use alloy_primitives::Address;
use thiserror::Error;

use crate::{
    bstock_market::{BNB_CHAIN_ID, BstockCatalog, CatalogError},
    liquidity::{LiquidityOpportunity, LiquidityOpportunityReader, LiquidityProviderError},
};

#[derive(Debug, Error, Eq, PartialEq)]
pub enum LiquidityServiceError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Provider(#[from] LiquidityProviderError),
    #[error("the token is not a supported bStock")]
    UnsupportedBstock,
}

pub struct LiquidityService {
    catalog: Arc<dyn BstockCatalog>,
    opportunities: Arc<dyn LiquidityOpportunityReader>,
}

impl LiquidityService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        opportunities: Arc<dyn LiquidityOpportunityReader>,
    ) -> Self {
        Self {
            catalog,
            opportunities,
        }
    }

    /// Returns current Binance-supported liquidity opportunities containing the requested bStock.
    ///
    /// # Errors
    ///
    /// Returns [`LiquidityServiceError::UnsupportedBstock`] when the token is outside Stockfront's
    /// bStock catalog, or a provider error when current data cannot be read safely.
    pub async fn opportunities(
        &self,
        bstock_token: Address,
    ) -> Result<Vec<LiquidityOpportunity>, LiquidityServiceError> {
        let supported = self
            .catalog
            .list_bstocks()
            .await?
            .into_iter()
            .any(|bstock| bstock.id.chain_id == BNB_CHAIN_ID && bstock.id.contract == bstock_token);
        if !supported {
            return Err(LiquidityServiceError::UnsupportedBstock);
        }

        self.opportunities
            .opportunities(bstock_token)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, address};
    use async_trait::async_trait;

    use crate::bstock_market::{BstockDetails, BstockId, BstockSummary};

    use super::*;

    struct FixedCatalog(Vec<BstockSummary>);

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(self.0.clone())
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    struct UnreachableReader;

    #[async_trait]
    impl LiquidityOpportunityReader for UnreachableReader {
        async fn opportunities(
            &self,
            _bstock_token: Address,
        ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError> {
            panic!("the provider must not be called for an unsupported token")
        }
    }

    #[tokio::test]
    async fn rejects_tokens_outside_the_bstock_catalog_before_provider_discovery() {
        let service = LiquidityService::new(
            Arc::new(FixedCatalog(Vec::new())),
            Arc::new(UnreachableReader),
        );

        let result = service
            .opportunities(address!("5b1910eAaD6450E50f816082Aa078C41F10C292f"))
            .await;

        assert_eq!(result, Err(LiquidityServiceError::UnsupportedBstock));
    }
}
