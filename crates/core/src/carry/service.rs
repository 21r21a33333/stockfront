use std::sync::Arc;

use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    bstock_borrowing::{
        BorrowingMarket, BorrowingMarketQuery, BorrowingService, BorrowingServiceError,
    },
    carry::{
        CarryAccount, CarryActionPlan, CarryOpportunity, CarryOpportunityQuery,
        CarryOpportunityReason, CarryQualification, CarryWithdrawalAmount, EarnActionGateway,
        EarnDepositCommand, EarnOpportunity, EarnProviderError, EarnRateType, EarnReader,
        EarnRedemptionClaimCommand, EarnWithdrawalCommand, PrepareCarryDepositRequest,
        PrepareCarryRedemptionClaimRequest, PrepareCarryWithdrawalRequest, PreparedCarryDeposit,
    },
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Debug)]
pub struct CarryPolicy {
    reviewed_investment_ids: Vec<String>,
    minimum_base_spread_bps: u32,
}

impl CarryPolicy {
    #[must_use]
    pub fn new(reviewed_investment_ids: Vec<String>, minimum_base_spread_bps: u32) -> Self {
        let mut reviewed_investment_ids = reviewed_investment_ids
            .into_iter()
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        reviewed_investment_ids.sort_unstable();
        reviewed_investment_ids.dedup();
        Self {
            reviewed_investment_ids,
            minimum_base_spread_bps,
        }
    }
}

#[derive(Debug, Error)]
pub enum CarryServiceError {
    #[error(transparent)]
    Borrowing(#[from] BorrowingServiceError),
    #[error(transparent)]
    EarnProvider(#[from] EarnProviderError),
    #[error("carry positions could not be read: {0}")]
    Positions(EarnProviderError),
    #[error("carry deposit amount must be greater than zero")]
    InvalidDepositAmount,
    #[error("carry withdrawal amount must be greater than zero")]
    InvalidWithdrawalAmount,
    #[error("the earn opportunity was not found")]
    OpportunityNotFound,
    #[error("the earn opportunity is not currently eligible")]
    OpportunityUnavailable,
}

pub struct CarryService {
    borrowing: Arc<BorrowingService>,
    earn_reader: Arc<dyn EarnReader>,
    earn_gateway: Arc<dyn EarnActionGateway>,
    policy: CarryPolicy,
}

impl CarryService {
    pub fn new(
        borrowing: Arc<BorrowingService>,
        earn_reader: Arc<dyn EarnReader>,
        earn_gateway: Arc<dyn EarnActionGateway>,
        policy: CarryPolicy,
    ) -> Self {
        Self {
            borrowing,
            earn_reader,
            earn_gateway,
            policy,
        }
    }

    /// Returns current earn routes assessed against the matching Venus borrow market.
    ///
    /// # Errors
    ///
    /// Returns [`CarryServiceError`] when the requested market is unsupported or a
    /// provider cannot supply current data.
    pub async fn opportunities(
        &self,
        query: CarryOpportunityQuery,
    ) -> Result<Vec<CarryOpportunity>, CarryServiceError> {
        let markets = self
            .borrowing
            .markets(BorrowingMarketQuery {
                collateral_token: Some(query.collateral_token),
                debt_token: Some(query.debt_token),
            })
            .await?;
        let earn_opportunities = self.earn_reader.opportunities(query.debt_token).await?;
        let mut opportunities = markets
            .iter()
            .flat_map(|market| {
                earn_opportunities
                    .iter()
                    .map(|earn| assess(market, earn, &self.policy))
            })
            .collect::<Vec<_>>();
        opportunities.sort_by(|left, right| {
            right
                .base_spread_percent
                .cmp(&left.base_spread_percent)
                .then_with(|| left.investment_id.cmp(&right.investment_id))
        });
        Ok(opportunities)
    }

    /// Returns the wallet's BNB Chain earn positions.
    ///
    /// # Errors
    ///
    /// Returns [`CarryServiceError`] when the earn provider cannot supply current position data.
    pub async fn positions(
        &self,
        wallet: WalletAddress,
    ) -> Result<CarryAccount, CarryServiceError> {
        self.earn_reader
            .positions(wallet)
            .await
            .map_err(CarryServiceError::Positions)
    }

    /// Returns a simulated, unsigned deposit plan for a currently eligible earn opportunity.
    ///
    /// # Errors
    ///
    /// Returns [`CarryServiceError`] when the amount or opportunity is invalid, the route is no
    /// longer eligible, or a provider cannot prepare the transaction plan.
    pub async fn prepare_deposit(
        &self,
        request: &PrepareCarryDepositRequest,
    ) -> Result<PreparedCarryDeposit, CarryServiceError> {
        if request.amount <= Decimal::ZERO {
            return Err(CarryServiceError::InvalidDepositAmount);
        }
        let earn = self
            .earn_reader
            .opportunity(&request.investment_id)
            .await?
            .ok_or(CarryServiceError::OpportunityNotFound)?;
        let markets = self
            .borrowing
            .markets(BorrowingMarketQuery {
                collateral_token: Some(request.collateral_token),
                debt_token: Some(earn.asset.contract),
            })
            .await?;
        let opportunity = markets
            .iter()
            .map(|market| assess(market, &earn, &self.policy))
            .find(|opportunity| opportunity.qualification == CarryQualification::Eligible)
            .ok_or(CarryServiceError::OpportunityUnavailable)?;
        let plan = self
            .earn_gateway
            .prepare_deposit(&EarnDepositCommand {
                wallet: request.wallet,
                investment_id: opportunity.investment_id.clone(),
                asset: opportunity.asset.contract,
                amount: request.amount,
            })
            .await?;

        Ok(PreparedCarryDeposit {
            opportunity,
            transactions: plan.transactions,
            preview: plan.preview,
        })
    }

    /// Returns a simulated, unsigned withdrawal plan for a BNB Chain earn opportunity.
    ///
    /// # Errors
    ///
    /// Returns [`CarryServiceError`] when the amount or investment is invalid, or when the earn
    /// provider cannot prepare the transaction plan.
    pub async fn prepare_withdrawal(
        &self,
        request: &PrepareCarryWithdrawalRequest,
    ) -> Result<CarryActionPlan, CarryServiceError> {
        if matches!(
            request.amount,
            CarryWithdrawalAmount::Exact { value, .. } if value <= Decimal::ZERO
        ) {
            return Err(CarryServiceError::InvalidWithdrawalAmount);
        }
        self.earn_reader
            .opportunity(&request.investment_id)
            .await?
            .ok_or(CarryServiceError::OpportunityNotFound)?;
        self.earn_gateway
            .prepare_withdrawal(&EarnWithdrawalCommand {
                wallet: request.wallet,
                investment_id: request.investment_id.clone(),
                amount: request.amount.clone(),
            })
            .await
            .map_err(Into::into)
    }

    /// Returns a simulated, unsigned claim plan for a delayed redemption.
    ///
    /// # Errors
    ///
    /// Returns [`CarryServiceError`] when the earn provider cannot prepare the claim.
    pub async fn prepare_redemption_claim(
        &self,
        request: &PrepareCarryRedemptionClaimRequest,
    ) -> Result<CarryActionPlan, CarryServiceError> {
        self.earn_gateway
            .prepare_redemption_claim(&EarnRedemptionClaimCommand {
                wallet: request.wallet,
                investment_id: request.investment_id.clone(),
                redemption_id: request.redemption_id.clone(),
            })
            .await
            .map_err(Into::into)
    }
}

fn assess(
    market: &BorrowingMarket,
    earn: &EarnOpportunity,
    policy: &CarryPolicy,
) -> CarryOpportunity {
    let base_spread_percent = (earn.rate_type == EarnRateType::Apy).then(|| {
        Decimal::from(earn.base_rate_bps) / Decimal::from(100) - market.borrow_apy_percent
    });
    let minimum_base_spread_percent =
        Decimal::from(policy.minimum_base_spread_bps) / Decimal::from(100);
    let mut reasons = Vec::new();

    if !earn.investable {
        reasons.push(CarryOpportunityReason::EarnUnavailable);
    }
    if !market.borrow_available
        || !market.oracle_valid
        || market.available_borrow_liquidity <= Decimal::ZERO
    {
        reasons.push(CarryOpportunityReason::BorrowUnavailable);
    }
    if earn.rate_type != EarnRateType::Apy {
        reasons.push(CarryOpportunityReason::UnsupportedRateType);
    }
    if base_spread_percent.is_some_and(|spread| spread < minimum_base_spread_percent) {
        reasons.push(CarryOpportunityReason::InsufficientBaseSpread);
    }

    let qualification = if reasons.is_empty() {
        if policy.reviewed_investment_ids.contains(&earn.investment_id) {
            CarryQualification::Eligible
        } else {
            reasons.push(CarryOpportunityReason::UnreviewedInvestment);
            CarryQualification::ReviewRequired
        }
    } else {
        CarryQualification::Unavailable
    };

    CarryOpportunity {
        investment_id: earn.investment_id.clone(),
        investment_name: earn.investment_name.clone(),
        protocol_id: earn.protocol_id.clone(),
        protocol_name: earn.protocol_name.clone(),
        protocol_logo_url: earn.protocol_logo_url.clone(),
        asset: earn.asset.clone(),
        rate_type: earn.rate_type,
        base_earn_rate_bps: earn.base_rate_bps,
        base_earn_rate_display: earn.base_rate_display.clone(),
        borrow_apy_percent: market.borrow_apy_percent,
        base_spread_percent,
        reported_tvl_usd: earn.reported_tvl_usd,
        available_borrow_liquidity: market.available_borrow_liquidity,
        qualification,
        reasons,
        observed_at_ms: market.observed_at_ms.min(earn.observed_at_ms),
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, B256, address};
    use async_trait::async_trait;
    use rust_decimal::Decimal;

    use super::*;
    use crate::{
        bstock_borrowing::{
            BorrowingAccount, BorrowingActionGateway, BorrowingPolicy, BorrowingProviderError,
            BorrowingReader, BorrowingToken, BorrowingTransactionStatus, BorrowingVenue,
            PrepareBorrowRequest, PrepareCollateralRequest, PrepareRepaymentRequest,
            PrepareSupplyRequest, PrepareWithdrawalRequest, PreparedBorrowingAction,
        },
        bstock_market::{
            AssetKind, BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockSummary,
            CatalogError,
        },
        carry::{CarryActionPlan, CarryActionPreview, RedemptionDelay},
        wallet_portfolio::WalletAddress,
    };

    const INVESTMENT_ID: &str = "reviewed-investment";

    #[test]
    fn computes_base_spread_from_integer_basis_points() {
        let opportunity = assess(
            &market(Decimal::new(425, 2), true, Decimal::new(1_000, 0)),
            &earn(725, true),
            &CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100),
        );

        assert_eq!(opportunity.base_spread_percent, Some(Decimal::new(3, 0)));
        assert_eq!(opportunity.qualification, CarryQualification::Eligible);
        assert!(opportunity.reasons.is_empty());
    }

    #[test]
    fn requires_review_only_after_market_and_spread_checks_pass() {
        let policy = CarryPolicy::new(Vec::new(), 100);
        let opportunity = assess(
            &market(Decimal::new(4, 0), true, Decimal::new(1_000, 0)),
            &earn(600, true),
            &policy,
        );

        assert_eq!(
            opportunity.qualification,
            CarryQualification::ReviewRequired
        );
        assert_eq!(
            opportunity.reasons,
            vec![CarryOpportunityReason::UnreviewedInvestment]
        );
    }

    #[test]
    fn marks_unavailable_earn_borrow_and_spread_conditions() {
        let policy = CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100);
        let opportunity = assess(
            &market(Decimal::new(6, 0), false, Decimal::ZERO),
            &earn(500, false),
            &policy,
        );

        assert_eq!(opportunity.qualification, CarryQualification::Unavailable);
        assert_eq!(
            opportunity.reasons,
            vec![
                CarryOpportunityReason::EarnUnavailable,
                CarryOpportunityReason::BorrowUnavailable,
                CarryOpportunityReason::InsufficientBaseSpread,
            ]
        );
    }

    #[test]
    fn apr_opportunity_is_visible_but_never_compared_with_borrow_apy() {
        let policy = CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100);
        let mut earn = earn(900, true);
        earn.rate_type = EarnRateType::Apr;

        let opportunity = assess(
            &market(Decimal::new(4, 0), true, Decimal::new(1_000, 0)),
            &earn,
            &policy,
        );

        assert_eq!(opportunity.base_spread_percent, None);
        assert_eq!(opportunity.qualification, CarryQualification::Unavailable);
        assert_eq!(
            opportunity.reasons,
            vec![CarryOpportunityReason::UnsupportedRateType]
        );
    }

    #[tokio::test]
    async fn eligible_deposit_uses_the_investment_asset_and_current_opportunity() {
        let service = service(CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100));
        let wallet = wallet();
        let deposit = service
            .prepare_deposit(&PrepareCarryDepositRequest {
                wallet,
                collateral_token: Address::with_last_byte(1),
                investment_id: INVESTMENT_ID.to_owned(),
                amount: Decimal::new(25, 0),
            })
            .await
            .unwrap_or_else(|error| panic!("eligible deposit rejected: {error}"));

        assert_eq!(deposit.opportunity.investment_id, INVESTMENT_ID);
        assert_eq!(
            deposit.opportunity.qualification,
            CarryQualification::Eligible
        );
        assert_eq!(
            deposit.opportunity.asset.contract,
            Address::with_last_byte(3)
        );
    }

    #[tokio::test]
    async fn deposit_rejects_invalid_amount_and_unreviewed_opportunity() {
        let service = service(CarryPolicy::new(Vec::new(), 100));
        let request = PrepareCarryDepositRequest {
            wallet: wallet(),
            collateral_token: Address::with_last_byte(1),
            investment_id: INVESTMENT_ID.to_owned(),
            amount: Decimal::ZERO,
        };

        assert!(matches!(
            service.prepare_deposit(&request).await,
            Err(CarryServiceError::InvalidDepositAmount)
        ));
        let request = PrepareCarryDepositRequest {
            amount: Decimal::ONE,
            ..request
        };
        assert!(matches!(
            service.prepare_deposit(&request).await,
            Err(CarryServiceError::OpportunityUnavailable)
        ));
    }

    #[tokio::test]
    async fn positions_are_not_scoped_by_the_deposit_allowlist() {
        let service = service(CarryPolicy::new(Vec::new(), 100));

        let account = service
            .positions(wallet())
            .await
            .unwrap_or_else(|error| panic!("carry positions failed: {error}"));

        assert_eq!(account.chain_id, BNB_CHAIN_ID);
        assert_eq!(account.observed_at_ms, 2_000);
    }

    #[tokio::test]
    async fn withdrawal_supports_exact_and_full_amounts_without_the_deposit_allowlist() {
        let service = service(CarryPolicy::new(Vec::new(), 100));
        let exact = service
            .prepare_withdrawal(&PrepareCarryWithdrawalRequest {
                wallet: wallet(),
                investment_id: INVESTMENT_ID.to_owned(),
                amount: CarryWithdrawalAmount::Exact {
                    token: Address::with_last_byte(3),
                    value: Decimal::new(25, 0),
                },
            })
            .await
            .unwrap_or_else(|error| panic!("exact withdrawal rejected: {error}"));
        let full = service
            .prepare_withdrawal(&PrepareCarryWithdrawalRequest {
                wallet: wallet(),
                investment_id: INVESTMENT_ID.to_owned(),
                amount: CarryWithdrawalAmount::Full,
            })
            .await
            .unwrap_or_else(|error| panic!("full withdrawal rejected: {error}"));

        assert_eq!(exact.preview.redemption_delay, None);
        assert_eq!(
            full.preview.redemption_delay,
            Some(RedemptionDelay {
                min_days: Decimal::new(7, 0),
                max_days: Decimal::new(10, 0),
            })
        );
    }

    #[tokio::test]
    async fn withdrawal_rejects_non_positive_exact_amount_and_unknown_investment() {
        let service = service(CarryPolicy::new(Vec::new(), 100));
        for value in [Decimal::ZERO, Decimal::NEGATIVE_ONE] {
            let request = PrepareCarryWithdrawalRequest {
                wallet: wallet(),
                investment_id: INVESTMENT_ID.to_owned(),
                amount: CarryWithdrawalAmount::Exact {
                    token: Address::with_last_byte(3),
                    value,
                },
            };
            assert!(matches!(
                service.prepare_withdrawal(&request).await,
                Err(CarryServiceError::InvalidWithdrawalAmount)
            ));
        }
        let request = PrepareCarryWithdrawalRequest {
            wallet: wallet(),
            investment_id: "unknown".to_owned(),
            amount: CarryWithdrawalAmount::Full,
        };
        assert!(matches!(
            service.prepare_withdrawal(&request).await,
            Err(CarryServiceError::OpportunityNotFound)
        ));
    }

    #[tokio::test]
    async fn redemption_claim_passes_only_provider_owned_identifiers() {
        let service = service(CarryPolicy::new(Vec::new(), 100));

        let plan = service
            .prepare_redemption_claim(&PrepareCarryRedemptionClaimRequest {
                wallet: wallet(),
                investment_id: INVESTMENT_ID.to_owned(),
                redemption_id: "redemption".to_owned(),
            })
            .await
            .unwrap_or_else(|error| panic!("redemption claim rejected: {error}"));

        assert!(plan.transactions.is_empty());
    }

    struct FixedCatalog;

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(vec![BstockSummary {
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
            }])
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    struct FixedBorrowingReader;

    #[async_trait]
    impl BorrowingReader for FixedBorrowingReader {
        async fn read_markets(
            &self,
            collateral_assets: &[BstockId],
            debt_tokens: &[Address],
        ) -> Result<Vec<BorrowingMarket>, BorrowingProviderError> {
            assert_eq!(collateral_assets[0].contract, Address::with_last_byte(1));
            assert_eq!(debt_tokens, &[Address::with_last_byte(3)]);
            Ok(vec![market(
                Decimal::new(4, 0),
                true,
                Decimal::new(1_000, 0),
            )])
        }

        async fn read_account(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
    }

    struct UnusedBorrowingGateway;

    #[async_trait]
    impl BorrowingActionGateway for UnusedBorrowingGateway {
        async fn prepare_supply(
            &self,
            _request: &PrepareSupplyRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn prepare_collateral(
            &self,
            _request: &PrepareCollateralRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn prepare_borrow(
            &self,
            _request: &PrepareBorrowRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn prepare_repayment_approval(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn prepare_repayment(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn prepare_withdrawal(
            &self,
            _request: &PrepareWithdrawalRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn transaction_status(
            &self,
            _transaction_hash: B256,
        ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
    }

    struct FixedEarn;

    #[async_trait]
    impl EarnReader for FixedEarn {
        async fn opportunities(
            &self,
            _asset: Address,
        ) -> Result<Vec<EarnOpportunity>, EarnProviderError> {
            Ok(vec![earn(700, true)])
        }

        async fn opportunity(
            &self,
            investment_id: &str,
        ) -> Result<Option<EarnOpportunity>, EarnProviderError> {
            Ok((investment_id == INVESTMENT_ID).then(|| earn(700, true)))
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<CarryAccount, EarnProviderError> {
            Ok(CarryAccount {
                chain_id: BNB_CHAIN_ID,
                positions: Vec::new(),
                total_value_usd: Decimal::ZERO,
                observed_at_ms: 2_000,
            })
        }
    }

    #[async_trait]
    impl EarnActionGateway for FixedEarn {
        async fn prepare_deposit(
            &self,
            command: &EarnDepositCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            assert_eq!(command.wallet, wallet());
            assert_eq!(command.investment_id, INVESTMENT_ID);
            assert_eq!(command.asset, Address::with_last_byte(3));
            assert_eq!(command.amount, Decimal::new(25, 0));
            Ok(CarryActionPlan {
                transactions: Vec::new(),
                preview: CarryActionPreview::default(),
            })
        }

        async fn prepare_withdrawal(
            &self,
            command: &EarnWithdrawalCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            assert_eq!(command.wallet, wallet());
            assert_eq!(command.investment_id, INVESTMENT_ID);
            let redemption_delay = match command.amount {
                CarryWithdrawalAmount::Exact { token, value } => {
                    assert_eq!(token, Address::with_last_byte(3));
                    assert_eq!(value, Decimal::new(25, 0));
                    None
                }
                CarryWithdrawalAmount::Full => Some(RedemptionDelay {
                    min_days: Decimal::new(7, 0),
                    max_days: Decimal::new(10, 0),
                }),
            };
            Ok(CarryActionPlan {
                transactions: Vec::new(),
                preview: CarryActionPreview {
                    redemption_delay,
                    ..CarryActionPreview::default()
                },
            })
        }

        async fn prepare_redemption_claim(
            &self,
            command: &EarnRedemptionClaimCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            assert_eq!(command.wallet, wallet());
            assert_eq!(command.investment_id, INVESTMENT_ID);
            assert_eq!(command.redemption_id, "redemption");
            Ok(CarryActionPlan {
                transactions: Vec::new(),
                preview: CarryActionPreview::default(),
            })
        }
    }

    fn service(policy: CarryPolicy) -> CarryService {
        let earn = Arc::new(FixedEarn);
        let borrowing = Arc::new(BorrowingService::new(
            Arc::new(FixedCatalog),
            Arc::new(FixedBorrowingReader),
            Arc::new(UnusedBorrowingGateway),
            BorrowingPolicy::new(vec![Address::with_last_byte(3)])
                .unwrap_or_else(|error| panic!("valid borrowing policy rejected: {error}")),
        ));
        CarryService::new(borrowing, earn.clone(), earn, policy)
    }

    fn wallet() -> WalletAddress {
        WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"))
    }

    fn market(
        borrow_apy_percent: Decimal,
        borrow_available: bool,
        available_borrow_liquidity: Decimal,
    ) -> BorrowingMarket {
        BorrowingMarket {
            venue: BorrowingVenue::Venus,
            chain_id: 56,
            collateral: token(
                address!("0000000000000000000000000000000000000001"),
                "TSLAB",
            ),
            collateral_market: address!("0000000000000000000000000000000000000002"),
            debt: token(address!("0000000000000000000000000000000000000003"), "USDT"),
            debt_market: address!("0000000000000000000000000000000000000004"),
            collateral_factor: Decimal::new(6, 1),
            liquidation_threshold: Decimal::new(7, 1),
            remaining_supply_capacity: Decimal::new(1_000, 0),
            available_borrow_liquidity,
            borrow_apy_percent,
            supply_available: true,
            borrow_available,
            oracle_valid: true,
            unavailable_reasons: Vec::new(),
            observed_at_ms: 2_000,
        }
    }

    fn earn(base_rate_bps: i64, investable: bool) -> EarnOpportunity {
        EarnOpportunity {
            investment_id: INVESTMENT_ID.to_owned(),
            investment_name: "USDT Earn".to_owned(),
            protocol_id: "venus".to_owned(),
            protocol_name: "Venus".to_owned(),
            protocol_logo_url: None,
            asset: token(address!("0000000000000000000000000000000000000003"), "USDT"),
            rate_type: EarnRateType::Apy,
            base_rate_bps,
            base_rate_display: "7.25%".to_owned(),
            reported_tvl_usd: Some(Decimal::new(5_000, 0)),
            investable,
            observed_at_ms: 3_000,
        }
    }

    fn token(contract: alloy_primitives::Address, symbol: &str) -> BorrowingToken {
        BorrowingToken {
            contract,
            symbol: symbol.to_owned(),
        }
    }
}
