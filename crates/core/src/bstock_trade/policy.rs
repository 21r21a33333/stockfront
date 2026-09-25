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
}
