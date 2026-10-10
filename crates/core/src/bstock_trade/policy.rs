use std::collections::HashSet;

use alloy_primitives::Address;
use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TradePolicyError {
    #[error("trade price-impact limit must be between 1 and 10000 basis points")]
    InvalidPriceImpactLimit,
    #[error("trade slippage must not exceed 10000 basis points")]
    InvalidSlippage,
    #[error("at least one settlement token must be configured")]
    MissingSettlementToken,
    #[error("the zero address cannot be a settlement token")]
    InvalidSettlementToken,
}

/// Whether a trade between two settlement tokens may be offered.
///
/// The venue behind a trade is a generic swap aggregator and quotes such a pair
/// happily, but offering it widens the product's vocabulary, so it is a
/// deployment decision rather than a provider capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementPairs {
    Rejected,
    Allowed,
}

/// What a sell/buy pair is, once the policy recognises it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TradePair {
    /// The bStock side, which still has to exist in the catalog.
    Bstock(Address),
    /// Two settlement tokens, which no catalog covers.
    Settlement,
}

#[derive(Clone)]
pub struct TradePolicy {
    pub(super) settlement_tokens: HashSet<Address>,
    max_price_impact_percent: Decimal,
    pub(super) slippage_percent: Decimal,
    settlement_pairs: SettlementPairs,
}

impl TradePolicy {
    /// Creates the policy enforced for every supported asset trade.
    ///
    /// # Errors
    ///
    /// Returns [`TradePolicyError`] when limits or settlement tokens are unsafe.
    pub fn new(
        settlement_tokens: impl IntoIterator<Item = Address>,
        max_price_impact_bps: u32,
        slippage_bps: u32,
        settlement_pairs: SettlementPairs,
    ) -> Result<Self, TradePolicyError> {
        let settlement_tokens = settlement_tokens.into_iter().collect::<HashSet<_>>();
        if settlement_tokens.is_empty() {
            return Err(TradePolicyError::MissingSettlementToken);
        }
        if settlement_tokens.contains(&Address::ZERO) {
            return Err(TradePolicyError::InvalidSettlementToken);
        }
        if !(1..=10_000).contains(&max_price_impact_bps) {
            return Err(TradePolicyError::InvalidPriceImpactLimit);
        }
        if slippage_bps > 10_000 {
            return Err(TradePolicyError::InvalidSlippage);
        }

        Ok(Self {
            settlement_tokens,
            max_price_impact_percent: Decimal::new(i64::from(max_price_impact_bps), 2),
            slippage_percent: Decimal::new(i64::from(slippage_bps), 2),
            settlement_pairs,
        })
    }

    pub(crate) fn accepts_price_impact(&self, price_impact_percent: Decimal) -> bool {
        price_impact_percent.abs() <= self.max_price_impact_percent
    }

    pub(crate) fn slippage_percent(&self) -> Decimal {
        self.slippage_percent
    }

    pub(crate) const fn offers_settlement_pairs(&self) -> bool {
        matches!(self.settlement_pairs, SettlementPairs::Allowed)
    }

    pub(crate) fn classify_pair(
        &self,
        sell_token: Address,
        buy_token: Address,
    ) -> Option<TradePair> {
        // Selling a token for itself is a no-op the venue would price anyway.
        if sell_token == buy_token {
            return None;
        }
        let sell_is_settlement = self.settlement_tokens.contains(&sell_token);
        let buy_is_settlement = self.settlement_tokens.contains(&buy_token);
        match (sell_is_settlement, buy_is_settlement) {
            (true, false) => Some(TradePair::Bstock(buy_token)),
            (false, true) => Some(TradePair::Bstock(sell_token)),
            (true, true) if self.offers_settlement_pairs() => Some(TradePair::Settlement),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_impact_limit_uses_the_magnitude_of_binances_signed_value() {
        let policy = policy(SettlementPairs::Rejected);

        assert!(policy.accepts_price_impact(Decimal::new(-100, 2)));
        assert!(policy.accepts_price_impact(Decimal::new(100, 2)));
        assert!(!policy.accepts_price_impact(Decimal::new(-101, 2)));
    }

    #[test]
    fn supported_pair_allows_both_trade_directions() {
        let policy = policy(SettlementPairs::Rejected);

        assert_eq!(
            policy.classify_pair(SETTLEMENT, BSTOCK),
            Some(TradePair::Bstock(BSTOCK))
        );
        assert_eq!(
            policy.classify_pair(BSTOCK, SETTLEMENT),
            Some(TradePair::Bstock(BSTOCK))
        );
        assert_eq!(policy.classify_pair(SETTLEMENT, OTHER_SETTLEMENT), None);
        assert_eq!(policy.classify_pair(BSTOCK, OTHER_ASSET), None);
        assert_eq!(policy.classify_pair(BSTOCK, BSTOCK), None);
    }

    #[test]
    fn settlement_pairs_are_offered_only_when_the_deployment_allows_them() {
        let allowed = policy(SettlementPairs::Allowed);

        // No bStock sits in the pair, so nothing is checked against the catalog.
        assert_eq!(
            allowed.classify_pair(SETTLEMENT, OTHER_SETTLEMENT),
            Some(TradePair::Settlement)
        );
        assert_eq!(
            allowed.classify_pair(SETTLEMENT, BSTOCK),
            Some(TradePair::Bstock(BSTOCK))
        );
        // Widening the pairs must not turn a no-op into a trade, nor admit an
        // asset the catalog has never heard of.
        assert_eq!(allowed.classify_pair(SETTLEMENT, SETTLEMENT), None);
        assert_eq!(allowed.classify_pair(BSTOCK, OTHER_ASSET), None);
    }

    const SETTLEMENT: Address = Address::with_last_byte(1);
    const BSTOCK: Address = Address::with_last_byte(2);
    const OTHER_SETTLEMENT: Address = Address::with_last_byte(3);
    const OTHER_ASSET: Address = Address::with_last_byte(4);

    fn policy(settlement_pairs: SettlementPairs) -> TradePolicy {
        TradePolicy::new([SETTLEMENT, OTHER_SETTLEMENT], 100, 50, settlement_pairs)
            .unwrap_or_else(|error| panic!("valid policy rejected: {error}"))
    }
}
