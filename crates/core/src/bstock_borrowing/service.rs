use std::sync::Arc;

use alloy_primitives::{Address, B256};
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    bstock_borrowing::{
        BorrowCapability, BorrowCapabilityRequest, BorrowingAccount, BorrowingActionGateway,
        BorrowingMarket, BorrowingMarketQuery, BorrowingProviderError, BorrowingReader,
        BorrowingTransactionStatus, PositionAmount, PrepareBorrowRequest, PrepareCollateralRequest,
        PrepareRepaymentRequest, PrepareSupplyRequest, PrepareWithdrawalRequest,
        PreparedBorrowingAction, SupplyMarket,
    },
    bstock_market::{BNB_CHAIN_ID, BstockCatalog, BstockId, CatalogError},
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowingPolicy {
    debt_tokens: Vec<Address>,
}

#[must_use]
pub fn minimum_borrow_health_factor() -> Decimal {
    Decimal::new(15, 1)
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

    #[must_use]
    pub fn debt_tokens(&self) -> &[Address] {
        &self.debt_tokens
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
    Provider(#[from] BorrowingProviderError),
    #[error("supply amount must be greater than zero")]
    InvalidSupplyAmount,
    #[error("borrow amount must be greater than zero")]
    InvalidBorrowAmount,
    #[error("repayment amount must be greater than zero")]
    InvalidRepaymentAmount,
    #[error("withdrawal amount must be greater than zero")]
    InvalidWithdrawalAmount,
    #[error("the collateral token is not a supported bStock")]
    UnsupportedCollateral,
    #[error("the debt token is not supported")]
    UnsupportedDebtToken,
    #[error("the borrow health factor is unavailable")]
    HealthFactorUnavailable,
    #[error("the borrow would reduce the health factor below the safety minimum")]
    UnsafeHealthFactor,
}

pub struct BorrowingService {
    catalog: Arc<dyn BstockCatalog>,
    reader: Arc<dyn BorrowingReader>,
    action_gateway: Arc<dyn BorrowingActionGateway>,
    policy: BorrowingPolicy,
}

impl BorrowingService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        reader: Arc<dyn BorrowingReader>,
        action_gateway: Arc<dyn BorrowingActionGateway>,
        policy: BorrowingPolicy,
    ) -> Self {
        Self {
            catalog,
            reader,
            action_gateway,
            policy,
        }
    }

    #[must_use]
    pub fn supported_debt_tokens(&self) -> &[Address] {
        self.policy.debt_tokens()
    }

    #[must_use]
    pub fn minimum_health_factor(&self) -> Decimal {
        minimum_borrow_health_factor()
    }

    /// Returns current account-wide safety for one exact borrow request.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when an asset is unsupported or live
    /// Venus account and market state cannot be read safely.
    pub async fn borrow_capability(
        &self,
        wallet: WalletAddress,
        pool: Address,
        collateral_token: Address,
        debt_token: Address,
        raw_amount: alloy_primitives::U256,
    ) -> Result<BorrowCapability, BorrowingServiceError> {
        if raw_amount.is_zero() {
            return Err(BorrowingServiceError::InvalidBorrowAmount);
        }
        self.ensure_supported_collateral(collateral_token).await?;
        if !self.policy.debt_tokens.contains(&debt_token) {
            return Err(BorrowingServiceError::UnsupportedDebtToken);
        }
        self.reader
            .read_borrow_capability(&BorrowCapabilityRequest {
                wallet,
                pool,
                collateral_token,
                debt_token,
                raw_amount,
                minimum_health_factor: minimum_borrow_health_factor(),
            })
            .await
            .map_err(Into::into)
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
    /// Returns [`BorrowingServiceError::Provider`] when current account state cannot be read.
    pub async fn account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingServiceError> {
        self.reader.read_account(wallet).await.map_err(Into::into)
    }

    /// Returns the current Venus supply capability for a supported bStock.
    ///
    /// `None` means the token is in Stockfront's bStock catalog but Venus does
    /// not currently expose a market for it.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the token is not a supported
    /// bStock or current Venus market data cannot be read.
    pub async fn supply_market(
        &self,
        token: Address,
    ) -> Result<Option<SupplyMarket>, BorrowingServiceError> {
        self.ensure_supported_collateral(token).await?;
        self.reader
            .read_supply_market(token)
            .await
            .map_err(Into::into)
    }

    /// Returns Binance's ordered wallet transactions for supplying a supported bStock to Venus.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the amount or collateral is unsupported, or Binance
    /// cannot prepare the transaction plan.
    pub async fn prepare_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        if request.amount <= Decimal::ZERO {
            return Err(BorrowingServiceError::InvalidSupplyAmount);
        }
        self.ensure_supported_collateral(request.collateral_token)
            .await?;
        self.action_gateway
            .prepare_supply(request)
            .await
            .map_err(Into::into)
    }

    /// Returns an unsigned Venus transaction that enables supplied bStock as collateral.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the token is unsupported or the action cannot be
    /// prepared. An already-enabled market returns an empty transaction plan.
    pub async fn prepare_collateral(
        &self,
        request: &PrepareCollateralRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        self.ensure_supported_collateral(request.collateral_token)
            .await?;
        self.action_gateway
            .prepare_collateral(request)
            .await
            .map_err(Into::into)
    }

    /// Returns an unsigned Venus transaction that borrows a supported debt token.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the amount or token is unsupported, or the action
    /// cannot be prepared.
    pub async fn prepare_borrow(
        &self,
        request: &PrepareBorrowRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        if request.amount <= Decimal::ZERO {
            return Err(BorrowingServiceError::InvalidBorrowAmount);
        }
        if !self.policy.debt_tokens.contains(&request.debt_token) {
            return Err(BorrowingServiceError::UnsupportedDebtToken);
        }
        let action = self.action_gateway.prepare_borrow(request).await?;
        let health_factor = action
            .preview
            .health_factor_after
            .ok_or(BorrowingServiceError::HealthFactorUnavailable)?;
        if health_factor < minimum_borrow_health_factor() {
            return Err(BorrowingServiceError::UnsafeHealthFactor);
        }
        Ok(action)
    }

    /// Returns an exact, finite token approval required by a Venus repayment.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the amount or debt token is unsupported, or the
    /// action cannot be prepared. A sufficient existing allowance returns an empty plan.
    pub async fn prepare_repayment_approval(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        self.validate_repayment(request)?;
        self.action_gateway
            .prepare_repayment_approval(request)
            .await
            .map_err(Into::into)
    }

    /// Returns an unsigned Venus repayment transaction.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the amount or debt token is unsupported, approval
    /// is required, or the action cannot be prepared.
    pub async fn prepare_repayment(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        self.validate_repayment(request)?;
        self.action_gateway
            .prepare_repayment(request)
            .await
            .map_err(Into::into)
    }

    /// Returns an unsigned Venus transaction for withdrawing supplied bStock.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError`] when the amount or collateral is unsupported, or Binance
    /// cannot prepare the transaction plan.
    pub async fn prepare_withdrawal(
        &self,
        request: &PrepareWithdrawalRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingServiceError> {
        if invalid_exact_amount(&request.amount) {
            return Err(BorrowingServiceError::InvalidWithdrawalAmount);
        }
        self.ensure_supported_collateral(request.collateral_token)
            .await?;
        self.action_gateway
            .prepare_withdrawal(request)
            .await
            .map_err(Into::into)
    }

    /// Returns Binance's current state for a submitted borrowing transaction.
    ///
    /// # Errors
    ///
    /// Returns [`BorrowingServiceError::Provider`] when Binance is unavailable or the transaction
    /// is unknown.
    pub async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<BorrowingTransactionStatus, BorrowingServiceError> {
        self.action_gateway
            .transaction_status(transaction_hash)
            .await
            .map_err(Into::into)
    }

    async fn ensure_supported_collateral(
        &self,
        collateral_token: Address,
    ) -> Result<(), BorrowingServiceError> {
        let id = BstockId {
            chain_id: BNB_CHAIN_ID,
            contract: collateral_token,
        };
        if self
            .catalog
            .list_bstocks()
            .await?
            .iter()
            .any(|asset| asset.id == id)
        {
            Ok(())
        } else {
            Err(BorrowingServiceError::UnsupportedCollateral)
        }
    }

    fn validate_repayment(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<(), BorrowingServiceError> {
        if invalid_exact_amount(&request.amount) {
            return Err(BorrowingServiceError::InvalidRepaymentAmount);
        }
        if !self.policy.debt_tokens.contains(&request.debt_token) {
            return Err(BorrowingServiceError::UnsupportedDebtToken);
        }
        Ok(())
    }
}

fn invalid_exact_amount(amount: &PositionAmount) -> bool {
    matches!(amount, PositionAmount::Exact { value } if *value <= Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::{
        bstock_borrowing::{
            BorrowingPreview, BorrowingProviderError, BorrowingTransactionState, BorrowingVenue,
        },
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
        async fn read_borrow_capability(
            &self,
            _request: &BorrowCapabilityRequest,
        ) -> Result<BorrowCapability, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn read_supply_market(
            &self,
            _token: Address,
        ) -> Result<Option<SupplyMarket>, BorrowingProviderError> {
            Ok(None)
        }

        async fn read_markets(
            &self,
            collateral_assets: &[BstockId],
            debt_tokens: &[Address],
        ) -> Result<Vec<BorrowingMarket>, BorrowingProviderError> {
            assert_eq!(collateral_assets.len(), 1);
            assert_eq!(debt_tokens, &[Address::with_last_byte(2)]);
            Ok(Vec::new())
        }

        async fn read_account(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingProviderError> {
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

    struct RecordingActionGateway;

    #[async_trait]
    impl BorrowingActionGateway for RecordingActionGateway {
        async fn prepare_supply(
            &self,
            request: &PrepareSupplyRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.collateral_token, Address::with_last_byte(1));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_collateral(
            &self,
            request: &PrepareCollateralRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.collateral_token, Address::with_last_byte(1));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_borrow(
            &self,
            request: &PrepareBorrowRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.debt_token, Address::with_last_byte(2));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview {
                    health_factor_after: Some(if request.amount == Decimal::new(99, 0) {
                        Decimal::new(14, 1)
                    } else {
                        Decimal::new(2, 0)
                    }),
                    ..BorrowingPreview::default()
                },
            })
        }

        async fn prepare_repayment_approval(
            &self,
            request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.debt_token, Address::with_last_byte(2));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_repayment(
            &self,
            request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.debt_token, Address::with_last_byte(2));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_withdrawal(
            &self,
            request: &PrepareWithdrawalRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.collateral_token, Address::with_last_byte(1));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn transaction_status(
            &self,
            _transaction_hash: B256,
        ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
            Ok(BorrowingTransactionStatus {
                state: BorrowingTransactionState::Succeeded,
                block_number: Some(1),
                executed_at_ms: Some(1),
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

    #[tokio::test]
    async fn supply_rejects_invalid_requests_before_provider_io() {
        let service = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(3))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        assert_eq!(
            service
                .prepare_supply(&PrepareSupplyRequest {
                    wallet,
                    collateral_token: Address::with_last_byte(1),
                    amount: Decimal::ZERO,
                })
                .await,
            Err(BorrowingServiceError::InvalidSupplyAmount)
        );
        assert_eq!(
            service
                .prepare_supply(&PrepareSupplyRequest {
                    wallet,
                    collateral_token: Address::with_last_byte(9),
                    amount: Decimal::ONE,
                })
                .await,
            Err(BorrowingServiceError::UnsupportedCollateral)
        );
    }

    #[tokio::test]
    async fn supported_supply_is_forwarded_to_the_action_gateway() {
        let supply = service()
            .prepare_supply(&PrepareSupplyRequest {
                wallet: WalletAddress::try_from(Address::with_last_byte(3))
                    .unwrap_or_else(|error| panic!("valid wallet rejected: {error}")),
                collateral_token: Address::with_last_byte(1),
                amount: Decimal::ONE,
            })
            .await
            .unwrap_or_else(|error| panic!("supported supply rejected: {error}"));

        assert!(supply.transactions.is_empty());
    }

    #[tokio::test]
    async fn collateral_and_borrow_reject_unsupported_requests_before_provider_io() {
        let service = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(3))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        assert_eq!(
            service
                .prepare_collateral(&PrepareCollateralRequest {
                    wallet,
                    collateral_token: Address::with_last_byte(9),
                })
                .await,
            Err(BorrowingServiceError::UnsupportedCollateral)
        );
        assert_eq!(
            service
                .prepare_borrow(&PrepareBorrowRequest {
                    wallet,
                    debt_token: Address::with_last_byte(2),
                    amount: Decimal::ZERO,
                })
                .await,
            Err(BorrowingServiceError::InvalidBorrowAmount)
        );
        assert_eq!(
            service
                .prepare_borrow(&PrepareBorrowRequest {
                    wallet,
                    debt_token: Address::with_last_byte(9),
                    amount: Decimal::ONE,
                })
                .await,
            Err(BorrowingServiceError::UnsupportedDebtToken)
        );
    }

    #[tokio::test]
    async fn supported_collateral_and_borrow_are_forwarded_to_the_action_gateway() {
        let service = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(3))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        let collateral = service
            .prepare_collateral(&PrepareCollateralRequest {
                wallet,
                collateral_token: Address::with_last_byte(1),
            })
            .await
            .unwrap_or_else(|error| panic!("supported collateral rejected: {error}"));
        let borrow = service
            .prepare_borrow(&PrepareBorrowRequest {
                wallet,
                debt_token: Address::with_last_byte(2),
                amount: Decimal::ONE,
            })
            .await
            .unwrap_or_else(|error| panic!("supported borrow rejected: {error}"));

        assert!(collateral.transactions.is_empty());
        assert!(borrow.transactions.is_empty());
    }

    #[tokio::test]
    async fn borrow_rejects_a_provider_action_below_the_safety_floor() {
        let error = service()
            .prepare_borrow(&PrepareBorrowRequest {
                wallet: WalletAddress::try_from(Address::with_last_byte(3))
                    .unwrap_or_else(|error| panic!("valid wallet rejected: {error}")),
                debt_token: Address::with_last_byte(2),
                amount: Decimal::new(99, 0),
            })
            .await;

        assert_eq!(error, Err(BorrowingServiceError::UnsafeHealthFactor));
    }

    #[tokio::test]
    async fn repayment_and_withdrawal_validate_exact_amounts_and_supported_tokens() {
        let service = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(3))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        assert_eq!(
            service
                .prepare_repayment(&PrepareRepaymentRequest {
                    wallet,
                    debt_token: Address::with_last_byte(2),
                    amount: PositionAmount::Exact {
                        value: Decimal::ZERO,
                    },
                })
                .await,
            Err(BorrowingServiceError::InvalidRepaymentAmount)
        );
        assert_eq!(
            service
                .prepare_withdrawal(&PrepareWithdrawalRequest {
                    wallet,
                    collateral_token: Address::with_last_byte(9),
                    amount: PositionAmount::Full,
                })
                .await,
            Err(BorrowingServiceError::UnsupportedCollateral)
        );
    }

    #[tokio::test]
    async fn full_repayment_and_withdrawal_are_forwarded() {
        let service = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(3))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        let approval = service
            .prepare_repayment_approval(&PrepareRepaymentRequest {
                wallet,
                debt_token: Address::with_last_byte(2),
                amount: PositionAmount::Full,
            })
            .await
            .unwrap_or_else(|error| panic!("supported repayment approval rejected: {error}"));
        let repayment = service
            .prepare_repayment(&PrepareRepaymentRequest {
                wallet,
                debt_token: Address::with_last_byte(2),
                amount: PositionAmount::Full,
            })
            .await
            .unwrap_or_else(|error| panic!("supported repayment rejected: {error}"));
        let withdrawal = service
            .prepare_withdrawal(&PrepareWithdrawalRequest {
                wallet,
                collateral_token: Address::with_last_byte(1),
                amount: PositionAmount::Full,
            })
            .await
            .unwrap_or_else(|error| panic!("supported withdrawal rejected: {error}"));

        assert!(approval.transactions.is_empty());
        assert!(repayment.transactions.is_empty());
        assert!(withdrawal.transactions.is_empty());
    }

    fn service() -> BorrowingService {
        BorrowingService::new(
            Arc::new(FixedCatalog(vec![summary()])),
            Arc::new(RecordingReader),
            Arc::new(RecordingActionGateway),
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
