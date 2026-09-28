use std::sync::Arc;

use alloy_primitives::Address;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    bstock_market::{BNB_CHAIN_ID, BstockCatalog, CatalogError},
    liquidity::{
        LiquidityActionGateway, LiquidityEntryCommand, LiquidityOpportunity,
        LiquidityOpportunityReader, LiquidityProviderError, PrepareLiquidityEntryRequest,
        PreparedLiquidityEntry,
    },
};

#[derive(Clone, Debug)]
pub struct LiquidityPolicy {
    reviewed_investment_ids: Vec<String>,
    slippage_bps: u32,
}

impl LiquidityPolicy {
    /// Creates the policy applied before preparing an LP entry.
    ///
    /// # Errors
    ///
    /// Returns [`LiquidityPolicyError::InvalidSlippage`] when slippage exceeds 100%.
    pub fn new(
        reviewed_investment_ids: Vec<String>,
        slippage_bps: u32,
    ) -> Result<Self, LiquidityPolicyError> {
        if slippage_bps > 10_000 {
            return Err(LiquidityPolicyError::InvalidSlippage);
        }
        let mut reviewed_investment_ids = reviewed_investment_ids
            .into_iter()
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        reviewed_investment_ids.sort_unstable();
        reviewed_investment_ids.dedup();
        Ok(Self {
            reviewed_investment_ids,
            slippage_bps,
        })
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum LiquidityPolicyError {
    #[error("liquidity slippage must not exceed 10000 basis points")]
    InvalidSlippage,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum LiquidityServiceError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Provider(#[from] LiquidityProviderError),
    #[error("the token is not a supported bStock")]
    UnsupportedBstock,
    #[error("liquidity input amount must be greater than zero")]
    InvalidAmount,
    #[error("liquidity price range must be greater than zero")]
    InvalidPriceRange,
    #[error("the liquidity opportunity is not approved")]
    OpportunityUnavailable,
}

pub struct LiquidityService {
    catalog: Arc<dyn BstockCatalog>,
    opportunities: Arc<dyn LiquidityOpportunityReader>,
    actions: Arc<dyn LiquidityActionGateway>,
    policy: LiquidityPolicy,
}

impl LiquidityService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        opportunities: Arc<dyn LiquidityOpportunityReader>,
        actions: Arc<dyn LiquidityActionGateway>,
        policy: LiquidityPolicy,
    ) -> Self {
        Self {
            catalog,
            opportunities,
            actions,
            policy,
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

    /// Returns paired token requirements and a simulated, unsigned LP-add plan.
    ///
    /// # Errors
    ///
    /// Returns [`LiquidityServiceError`] when the amount, range or investment is not allowed, or
    /// when the provider cannot prepare the action.
    pub async fn prepare_entry(
        &self,
        request: &PrepareLiquidityEntryRequest,
    ) -> Result<PreparedLiquidityEntry, LiquidityServiceError> {
        if request.input.amount <= Decimal::ZERO {
            return Err(LiquidityServiceError::InvalidAmount);
        }
        if request.price_range_percent <= Decimal::ZERO {
            return Err(LiquidityServiceError::InvalidPriceRange);
        }
        if !self
            .policy
            .reviewed_investment_ids
            .contains(&request.investment_id)
        {
            return Err(LiquidityServiceError::OpportunityUnavailable);
        }

        self.actions
            .prepare_entry(&LiquidityEntryCommand {
                wallet: request.wallet,
                investment_id: request.investment_id.clone(),
                input: request.input,
                price_range_percent: request.price_range_percent,
                slippage_bps: self.policy.slippage_bps,
            })
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

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

    struct UnreachableProvider;

    #[async_trait]
    impl LiquidityOpportunityReader for UnreachableProvider {
        async fn opportunities(
            &self,
            _bstock_token: Address,
        ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError> {
            panic!("the provider must not be called for an unsupported token")
        }
    }

    #[async_trait]
    impl LiquidityActionGateway for UnreachableProvider {
        async fn prepare_entry(
            &self,
            _command: &LiquidityEntryCommand,
        ) -> Result<PreparedLiquidityEntry, LiquidityProviderError> {
            panic!("the provider must not be called for an invalid entry")
        }
    }

    struct RecordingGateway(AtomicU32);

    #[async_trait]
    impl LiquidityActionGateway for RecordingGateway {
        async fn prepare_entry(
            &self,
            command: &LiquidityEntryCommand,
        ) -> Result<PreparedLiquidityEntry, LiquidityProviderError> {
            self.0.store(command.slippage_bps, Ordering::Relaxed);
            Ok(PreparedLiquidityEntry {
                required_amounts: vec![command.input],
                transactions: Vec::new(),
                preview: crate::prepared_action::ActionPreview::default(),
            })
        }
    }

    fn service(policy: LiquidityPolicy) -> LiquidityService {
        let provider = Arc::new(UnreachableProvider);
        LiquidityService::new(
            Arc::new(FixedCatalog(Vec::new())),
            provider.clone(),
            provider,
            policy,
        )
    }

    #[tokio::test]
    async fn rejects_tokens_outside_the_bstock_catalog_before_provider_discovery() {
        let service = service(
            LiquidityPolicy::new(Vec::new(), 100)
                .unwrap_or_else(|error| panic!("test policy must be valid: {error}")),
        );

        let result = service
            .opportunities(address!("5b1910eAaD6450E50f816082Aa078C41F10C292f"))
            .await;

        assert_eq!(result, Err(LiquidityServiceError::UnsupportedBstock));
    }

    #[tokio::test]
    async fn rejects_invalid_entries_before_the_provider() {
        let policy = LiquidityPolicy::new(vec!["reviewed".to_owned()], 100)
            .unwrap_or_else(|error| panic!("test policy must be valid: {error}"));
        let service = service(policy);
        let wallet = crate::wallet_portfolio::WalletAddress::try_from(Address::with_last_byte(1))
            .unwrap_or_else(|error| panic!("test wallet must be valid: {error}"));
        let request = PrepareLiquidityEntryRequest {
            wallet,
            investment_id: "reviewed".to_owned(),
            input: crate::liquidity::LiquidityAmount {
                token: Address::with_last_byte(2),
                amount: Decimal::ZERO,
            },
            price_range_percent: Decimal::ONE,
        };

        assert_eq!(
            service.prepare_entry(&request).await,
            Err(LiquidityServiceError::InvalidAmount)
        );

        let request = PrepareLiquidityEntryRequest {
            input: crate::liquidity::LiquidityAmount {
                amount: Decimal::ONE,
                ..request.input
            },
            price_range_percent: Decimal::ZERO,
            ..request
        };
        assert_eq!(
            service.prepare_entry(&request).await,
            Err(LiquidityServiceError::InvalidPriceRange)
        );
    }

    #[tokio::test]
    async fn rejects_unreviewed_investments_before_the_provider() {
        let service = service(
            LiquidityPolicy::new(Vec::new(), 100)
                .unwrap_or_else(|error| panic!("test policy must be valid: {error}")),
        );
        let request = PrepareLiquidityEntryRequest {
            wallet: crate::wallet_portfolio::WalletAddress::try_from(Address::with_last_byte(1))
                .unwrap_or_else(|error| panic!("test wallet must be valid: {error}")),
            investment_id: "unreviewed".to_owned(),
            input: crate::liquidity::LiquidityAmount {
                token: Address::with_last_byte(2),
                amount: Decimal::ONE,
            },
            price_range_percent: Decimal::ONE,
        };

        assert_eq!(
            service.prepare_entry(&request).await,
            Err(LiquidityServiceError::OpportunityUnavailable)
        );
    }

    #[tokio::test]
    async fn reviewed_entries_use_policy_slippage() {
        let reader = Arc::new(UnreachableProvider);
        let gateway = Arc::new(RecordingGateway(AtomicU32::new(0)));
        let service = LiquidityService::new(
            Arc::new(FixedCatalog(Vec::new())),
            reader,
            gateway.clone(),
            LiquidityPolicy::new(vec!["reviewed".to_owned()], 250)
                .unwrap_or_else(|error| panic!("test policy must be valid: {error}")),
        );
        let request = PrepareLiquidityEntryRequest {
            wallet: crate::wallet_portfolio::WalletAddress::try_from(Address::with_last_byte(1))
                .unwrap_or_else(|error| panic!("test wallet must be valid: {error}")),
            investment_id: "reviewed".to_owned(),
            input: crate::liquidity::LiquidityAmount {
                token: Address::with_last_byte(2),
                amount: Decimal::ONE,
            },
            price_range_percent: Decimal::new(5, 0),
        };

        let result = service.prepare_entry(&request).await;

        assert!(result.is_ok());
        assert_eq!(gateway.0.load(Ordering::Relaxed), 250);
    }
}
