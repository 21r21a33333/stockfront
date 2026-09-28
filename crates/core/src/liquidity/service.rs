use std::{collections::HashSet, sync::Arc};

use alloy_primitives::Address;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    bstock_market::{BNB_CHAIN_ID, BstockCatalog, CatalogError},
    liquidity::{
        LiquidityAccount, LiquidityActionGateway, LiquidityEntryCommand, LiquidityOpportunity,
        LiquidityProviderError, LiquidityReader, PrepareLiquidityClaimRequest,
        PrepareLiquidityEntryRequest, PreparedLiquidityAction, PreparedLiquidityEntry,
    },
    wallet_portfolio::WalletAddress,
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
    reader: Arc<dyn LiquidityReader>,
    actions: Arc<dyn LiquidityActionGateway>,
    policy: LiquidityPolicy,
}

impl LiquidityService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        reader: Arc<dyn LiquidityReader>,
        actions: Arc<dyn LiquidityActionGateway>,
        policy: LiquidityPolicy,
    ) -> Self {
        Self {
            catalog,
            reader,
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

        self.reader
            .opportunities(bstock_token)
            .await
            .map_err(Into::into)
    }

    /// Returns wallet-owned liquidity positions containing a supported bStock.
    ///
    /// # Errors
    ///
    /// Returns a provider or catalog error when current position data cannot be read safely.
    pub async fn positions(
        &self,
        wallet: WalletAddress,
    ) -> Result<LiquidityAccount, LiquidityServiceError> {
        let supported = self
            .catalog
            .list_bstocks()
            .await?
            .into_iter()
            .filter(|bstock| bstock.id.chain_id == BNB_CHAIN_ID)
            .map(|bstock| bstock.id.contract)
            .collect::<HashSet<_>>();
        let mut account = self.reader.positions(wallet).await?;
        account.positions.retain(|position| {
            position
                .assets
                .iter()
                .any(|asset| supported.contains(&asset.token.contract))
        });
        account.total_value_usd = account
            .positions
            .iter()
            .try_fold(Decimal::ZERO, |total, position| {
                total.checked_add(position.value_usd)
            })
            .ok_or(LiquidityProviderError::InvalidData)?;
        Ok(account)
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

    /// Returns a simulated, unsigned claim plan for an existing liquidity position.
    ///
    /// Claims remain available when an investment is no longer approved for new entries.
    ///
    /// # Errors
    ///
    /// Returns a provider error when the claim cannot be prepared.
    pub async fn prepare_claim(
        &self,
        request: &PrepareLiquidityClaimRequest,
    ) -> Result<PreparedLiquidityAction, LiquidityServiceError> {
        self.actions
            .prepare_claim(request)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use alloy_primitives::{Address, address};
    use async_trait::async_trait;

    use crate::{
        bstock_borrowing::{BorrowingAsset, BorrowingToken},
        bstock_market::{AssetKind, BstockDetails, BstockId, BstockSummary},
        liquidity::LiquidityPosition,
    };

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
    impl LiquidityReader for UnreachableProvider {
        async fn opportunities(
            &self,
            _bstock_token: Address,
        ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError> {
            panic!("the provider must not be called for an unsupported token")
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<LiquidityAccount, LiquidityProviderError> {
            panic!("the provider must not be called")
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

        async fn prepare_claim(
            &self,
            _request: &PrepareLiquidityClaimRequest,
        ) -> Result<PreparedLiquidityAction, LiquidityProviderError> {
            panic!("the provider must not be called")
        }
    }

    struct RecordingGateway {
        slippage_bps: AtomicU32,
        claim_called: AtomicBool,
    }

    #[async_trait]
    impl LiquidityActionGateway for RecordingGateway {
        async fn prepare_entry(
            &self,
            command: &LiquidityEntryCommand,
        ) -> Result<PreparedLiquidityEntry, LiquidityProviderError> {
            self.slippage_bps
                .store(command.slippage_bps, Ordering::Relaxed);
            Ok(PreparedLiquidityEntry {
                required_amounts: vec![command.input],
                transactions: Vec::new(),
                preview: crate::prepared_action::ActionPreview::default(),
            })
        }

        async fn prepare_claim(
            &self,
            _request: &PrepareLiquidityClaimRequest,
        ) -> Result<PreparedLiquidityAction, LiquidityProviderError> {
            self.claim_called.store(true, Ordering::Relaxed);
            Ok(PreparedLiquidityAction {
                transactions: Vec::new(),
                preview: crate::prepared_action::ActionPreview::default(),
            })
        }
    }

    struct FixedReader(LiquidityAccount);

    #[async_trait]
    impl LiquidityReader for FixedReader {
        async fn opportunities(
            &self,
            _bstock_token: Address,
        ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError> {
            Ok(Vec::new())
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<LiquidityAccount, LiquidityProviderError> {
            Ok(self.0.clone())
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
        let gateway = Arc::new(RecordingGateway {
            slippage_bps: AtomicU32::new(0),
            claim_called: AtomicBool::new(false),
        });
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
        assert_eq!(gateway.slippage_bps.load(Ordering::Relaxed), 250);
    }

    #[tokio::test]
    async fn claims_remain_available_without_entry_allowlisting() {
        let reader = Arc::new(UnreachableProvider);
        let gateway = Arc::new(RecordingGateway {
            slippage_bps: AtomicU32::new(0),
            claim_called: AtomicBool::new(false),
        });
        let service = LiquidityService::new(
            Arc::new(FixedCatalog(Vec::new())),
            reader,
            gateway.clone(),
            LiquidityPolicy::new(Vec::new(), 100)
                .unwrap_or_else(|error| panic!("test policy must be valid: {error}")),
        );
        let request = PrepareLiquidityClaimRequest {
            wallet: WalletAddress::try_from(Address::with_last_byte(1))
                .unwrap_or_else(|error| panic!("test wallet must be valid: {error}")),
            investment_id: "no-longer-reviewed".to_owned(),
            claim_type: crate::liquidity::LiquidityClaimType::LpFee,
            nft_id: "#1".to_owned(),
        };

        let result = service.prepare_claim(&request).await;

        assert!(result.is_ok());
        assert!(gateway.claim_called.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn positions_keep_supported_bstocks_without_entry_allowlisting() {
        let bstock = Address::with_last_byte(1);
        let reader = Arc::new(FixedReader(LiquidityAccount {
            chain_id: BNB_CHAIN_ID,
            positions: vec![
                position(bstock, "supported", 5),
                position(Address::with_last_byte(2), "unrelated", 7),
            ],
            total_value_usd: Decimal::new(12, 0),
            observed_at_ms: 42,
        }));
        let service = LiquidityService::new(
            Arc::new(FixedCatalog(vec![summary(bstock)])),
            reader,
            Arc::new(UnreachableProvider),
            LiquidityPolicy::new(Vec::new(), 100)
                .unwrap_or_else(|error| panic!("test policy must be valid: {error}")),
        );
        let wallet = WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("test wallet must be valid: {error}"));

        let account = service
            .positions(wallet)
            .await
            .unwrap_or_else(|error| panic!("supported position was rejected: {error}"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].position_id, "supported");
        assert_eq!(account.total_value_usd, Decimal::new(5, 0));
    }

    fn position(token: Address, position_id: &str, value: i64) -> LiquidityPosition {
        LiquidityPosition {
            position_id: position_id.to_owned(),
            investment_ids: vec!["unreviewed-investment".to_owned()],
            protocol_id: "pancakeswap4".to_owned(),
            protocol_name: "PancakeSwap".to_owned(),
            pool_address: Address::with_last_byte(3),
            nft_id: "1".to_owned(),
            value_usd: Decimal::new(value, 0),
            assets: vec![BorrowingAsset {
                token: BorrowingToken {
                    contract: token,
                    symbol: "TOKEN".to_owned(),
                },
                amount: Decimal::ONE,
                value_usd: Decimal::new(value, 0),
            }],
            claimable_assets: Vec::new(),
            fee_tier: Some(2_500),
            current_tick: Some(0),
            lower_tick: -100,
            upper_tick: 100,
            in_range: true,
            claim_actions: Vec::new(),
        }
    }

    fn summary(contract: Address) -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract,
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
