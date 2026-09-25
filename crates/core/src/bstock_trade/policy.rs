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

pub struct TradePolicy {
    pub(super) settlement_tokens: HashSet<Address>,
    max_price_impact_percent: Decimal,
    pub(super) slippage_percent: Decimal,
}

impl TradePolicy {
    /// Creates the policy enforced for every bStock trade.
    ///
    /// # Errors
    ///
    /// Returns [`TradePolicyError`] when limits or settlement tokens are unsafe.
    pub fn new(
        settlement_tokens: impl IntoIterator<Item = Address>,
        max_price_impact_bps: u32,
        slippage_bps: u32,
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
        })
    }

    pub(super) fn accepts_price_impact(&self, price_impact_percent: Decimal) -> bool {
        price_impact_percent.abs() <= self.max_price_impact_percent
    }

    pub(super) fn bstock_for_pair(
        &self,
        sell_token: Address,
        buy_token: Address,
    ) -> Option<Address> {
        let sell_is_settlement = self.settlement_tokens.contains(&sell_token);
        let buy_is_settlement = self.settlement_tokens.contains(&buy_token);
        match (sell_is_settlement, buy_is_settlement) {
            (true, false) => Some(buy_token),
            (false, true) => Some(sell_token),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_impact_limit_uses_the_magnitude_of_binances_signed_value() {
        let policy = TradePolicy::new([Address::with_last_byte(1)], 100, 50)
            .unwrap_or_else(|error| panic!("valid policy rejected: {error}"));

        assert!(policy.accepts_price_impact(Decimal::new(-100, 2)));
        assert!(policy.accepts_price_impact(Decimal::new(100, 2)));
        assert!(!policy.accepts_price_impact(Decimal::new(-101, 2)));
    }

    #[test]
    fn supported_pair_allows_both_trade_directions() {
        let settlement_token = Address::with_last_byte(1);
        let bstock = Address::with_last_byte(2);
        let other_settlement_token = Address::with_last_byte(3);
        let other_asset = Address::with_last_byte(4);
        let policy = TradePolicy::new([settlement_token, other_settlement_token], 100, 50)
            .unwrap_or_else(|error| panic!("valid policy rejected: {error}"));

        assert_eq!(
            policy.bstock_for_pair(settlement_token, bstock),
            Some(bstock)
        );
        assert_eq!(
            policy.bstock_for_pair(bstock, settlement_token),
            Some(bstock)
        );
        assert_eq!(
            policy.bstock_for_pair(settlement_token, other_settlement_token),
            None
        );
        assert_eq!(policy.bstock_for_pair(bstock, other_asset), None);
        assert_eq!(policy.bstock_for_pair(bstock, bstock), None);
    }
}
