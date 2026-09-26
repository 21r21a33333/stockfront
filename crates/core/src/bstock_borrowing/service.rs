use std::sync::Arc;

use alloy_primitives::Address;
use thiserror::Error;

use crate::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingMarket, BorrowingMarketQuery, BorrowingReadError,
        BorrowingReader,
    },
    bstock_market::{BstockCatalog, CatalogError},
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowingPolicy {
    debt_tokens: Vec<Address>,
}

impl BorrowingPolicy {
    /// Creates the debt-token allowlist used by the borrowing API.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingPolicyError::NoDebtTokens`] when no usable token is configured.
    pub fn new(mut debt_tokens: Vec<Address>) -> Result<Self, BorrowingPolicyError> {
        debt_tokens.retain(|token| *token != Address::ZERO);
        debt_tokens.sort_unstable();
        debt_tokens.dedup();
        if debt_tokens.is_empty() {
            return Err(BorrowingPolicyError::NoDebtTokens);
        }
        Ok(Self { debt_tokens })
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BorrowingPolicyError {
    #[error("at least one non-zero debt token must be configured")]
    NoDebtTokens,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum BorrowingServiceError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Reader(#[from] BorrowingReadError),
    #[error("the collateral token is not a supported bStock")]
    UnsupportedCollateral,
    #[error("the debt token is not supported")]
    UnsupportedDebtToken,
}

pub struct BorrowingService {
    catalog: Arc<dyn BstockCatalog>,
    reader: Arc<dyn BorrowingReader>,
    policy: BorrowingPolicy,
}

impl BorrowingService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        reader: Arc<dyn BorrowingReader>,
        policy: BorrowingPolicy,
    ) -> Self {
        Self {
            catalog,
            reader,
            policy,
        }
    }

    /// Returns Venus markets at the intersection of supported bStocks and debt assets.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when a filter is unsupported or current market data
    /// cannot be read.
    pub async fn markets(
        &self,
        query: BorrowingMarketQuery,
    ) -> Result<Vec<BorrowingMarket>, BorrowingServiceError> {
        let mut collateral_assets = self
            .catalog
            .list_bstocks()
            .await?
            .into_iter()
            .map(|asset| asset.id)
            .collect::<Vec<_>>();

        if let Some(collateral_token) = query.collateral_token {
            if !collateral_assets
                .iter()
                .any(|asset| asset.contract == collateral_token)
            {
                return Err(BorrowingServiceError::UnsupportedCollateral);
            }
            collateral_assets.retain(|asset| asset.contract == collateral_token);
        }

        let debt_tokens = match query.debt_token {
            Some(debt_token) if self.policy.debt_tokens.contains(&debt_token) => vec![debt_token],
            Some(_) => return Err(BorrowingServiceError::UnsupportedDebtToken),
            None => self.policy.debt_tokens.clone(),
        };

        self.reader
            .read_markets(&collateral_assets, &debt_tokens)
            .await
            .map_err(Into::into)
    }

    /// Returns the wallet's complete pooled Venus position.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError::Reader`] when current account state cannot be read.
    pub async fn account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingServiceError> {
        self.reader.read_account(wallet).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::{
        bstock_borrowing::{BorrowingReadError, BorrowingVenue},
        bstock_market::{AssetKind, BNB_CHAIN_ID, BstockDetails, BstockId, BstockSummary},
    };
    use rust_decimal::Decimal;

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

    struct RecordingReader;

    #[async_trait]
    impl BorrowingReader for RecordingReader {
        async fn read_markets(
            &self,
            collateral_assets: &[BstockId],
            debt_tokens: &[Address],
        ) -> Result<Vec<BorrowingMarket>, BorrowingReadError> {
            assert_eq!(collateral_assets.len(), 1);
            assert_eq!(debt_tokens, &[Address::with_last_byte(2)]);
            Ok(Vec::new())
        }

        async fn read_account(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingReadError> {
            Ok(BorrowingAccount {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                positions: Vec::new(),
                total_supplied_usd: Decimal::ZERO,
                total_borrowed_usd: Decimal::ZERO,
                health_factor: None,
                observed_at_ms: 1,
            })
        }
    }

    #[tokio::test]
    async fn market_filters_reject_unsupported_assets_before_provider_io() {
        let service = service();

        assert_eq!(
            service
                .markets(BorrowingMarketQuery {
                    collateral_token: Some(Address::with_last_byte(9)),
                    debt_token: None,
                })
                .await,
            Err(BorrowingServiceError::UnsupportedCollateral)
        );
        assert_eq!(
            service
                .markets(BorrowingMarketQuery {
                    collateral_token: None,
                    debt_token: Some(Address::with_last_byte(9)),
                })
                .await,
            Err(BorrowingServiceError::UnsupportedDebtToken)
        );
    }

    #[tokio::test]
    async fn supported_filters_are_forwarded_as_the_protocol_intersection() {
        let service = service();
        let markets = service
            .markets(BorrowingMarketQuery {
                collateral_token: Some(Address::with_last_byte(1)),
                debt_token: Some(Address::with_last_byte(2)),
            })
            .await
            .unwrap_or_else(|error| panic!("supported market rejected: {error}"));

        assert!(markets.is_empty());
    }

    fn service() -> BorrowingService {
        BorrowingService::new(
            Arc::new(FixedCatalog(vec![summary()])),
            Arc::new(RecordingReader),
            BorrowingPolicy::new(vec![Address::with_last_byte(2)])
                .unwrap_or_else(|error| panic!("valid policy rejected: {error}")),
        )
    }

    fn summary() -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: Address::with_last_byte(1),
            },
            token_name: "Stock".to_owned(),
            token_symbol: "STKB".to_owned(),
            token_logo_url: None,
            token_decimals: 18,
            underlying_ticker: "STK".to_owned(),
            underlying_name: "Stock".to_owned(),
            asset_kind: AssetKind::Stock,
            token_to_share_ratio: Decimal::ONE,
            token_price_usd: None,
            reference_price_usd: None,
            volume_24h_usd: None,
            market_cap_usd: None,
            market_status: None,
            tags: Vec::new(),
        }
    }
}
