use std::{collections::HashSet, sync::Arc};

use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    bstock_borrowing::{
        BorrowingMarket, BorrowingMarketQuery, BorrowingService, BorrowingServiceError,
    },
    carry::{
        CarryOpportunity, CarryOpportunityQuery, CarryOpportunityReason, CarryQualification,
        EarnOpportunity, EarnOpportunityProviderError, EarnOpportunityReader,
    },
};

#[derive(Clone, Debug)]
pub struct CarryPolicy {
    reviewed_investment_ids: HashSet<String>,
    minimum_base_spread_bps: u32,
}

impl CarryPolicy {
    #[must_use]
    pub fn new(reviewed_investment_ids: Vec<String>, minimum_base_spread_bps: u32) -> Self {
        let reviewed_investment_ids = reviewed_investment_ids
            .into_iter()
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
            .collect();
        Self {
            reviewed_investment_ids,
            minimum_base_spread_bps,
        }
    }
}

#[derive(Debug, Error)]
pub enum CarryOpportunityServiceError {
    #[error(transparent)]
    Borrowing(#[from] BorrowingServiceError),
    #[error(transparent)]
    EarnProvider(#[from] EarnOpportunityProviderError),
}

pub struct CarryOpportunityService {
    borrowing: Arc<BorrowingService>,
    earn_reader: Arc<dyn EarnOpportunityReader>,
    policy: CarryPolicy,
}

impl CarryOpportunityService {
    pub fn new(
        borrowing: Arc<BorrowingService>,
        earn_reader: Arc<dyn EarnOpportunityReader>,
        policy: CarryPolicy,
    ) -> Self {
        Self {
            borrowing,
            earn_reader,
            policy,
        }
    }

    /// Returns current earn routes assessed against the matching Venus borrow market.
    ///
    /// # Errors
    ///
    /// Returns [`CarryOpportunityServiceError`] when the requested market is unsupported or a
    /// provider cannot supply current data.
    pub async fn opportunities(
        &self,
        query: CarryOpportunityQuery,
    ) -> Result<Vec<CarryOpportunity>, CarryOpportunityServiceError> {
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
}

fn assess(
    market: &BorrowingMarket,
    earn: &EarnOpportunity,
    policy: &CarryPolicy,
) -> CarryOpportunity {
    let base_earn_rate_percent = Decimal::from(earn.base_rate_bps) / Decimal::from(100);
    let base_spread_percent = base_earn_rate_percent - market.borrow_apy_percent;
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
    if base_spread_percent < minimum_base_spread_percent {
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
    use alloy_primitives::address;
    use rust_decimal::Decimal;

    use super::*;
    use crate::{
        bstock_borrowing::{BorrowingToken, BorrowingVenue},
        carry::EarnRateType,
    };

    const INVESTMENT_ID: &str = "reviewed-investment";

    #[test]
    fn computes_base_spread_from_integer_basis_points() {
        let opportunity = assess(
            &market(Decimal::new(425, 2), true, Decimal::new(1_000, 0)),
            &earn(725, true),
            &CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100),
        );

        assert_eq!(opportunity.base_spread_percent, Decimal::new(3, 0));
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
