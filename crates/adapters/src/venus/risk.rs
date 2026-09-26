use std::collections::HashSet;

use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{BorrowingPreview, BorrowingProviderError},
    wallet_portfolio::WalletAddress,
};

use super::{client::VenusBorrowingAdapter, contracts::IVenusComptroller, markets::MarketState};

impl VenusBorrowingAdapter {
    pub(super) async fn apply_borrow_risk(
        &self,
        wallet: WalletAddress,
        debt_market: &MarketState,
        borrow_amount: Decimal,
        preview: &mut BorrowingPreview,
    ) -> Result<(), BorrowingProviderError> {
        let account = self.read_binance_account(wallet).await?;
        let positions = account
            .positions
            .iter()
            .filter(|position| position.pool == self.comptroller)
            .collect::<Vec<_>>();
        let supplied_tokens = positions
            .iter()
            .flat_map(|position| &position.supplied)
            .map(|asset| asset.token.contract)
            .collect::<HashSet<_>>();
        let markets = self.market_states(&supplied_tokens).await?;
        if markets.len() != supplied_tokens.len() {
            return Err(BorrowingProviderError::InvalidData);
        }

        let comptroller = IVenusComptroller::new(self.comptroller, &self.provider);
        let mut enabled_collateral = HashSet::new();
        for token in supplied_tokens {
            let market = markets
                .get(&token)
                .ok_or(BorrowingProviderError::InvalidData)?;
            let enabled = comptroller
                .checkMembership(wallet.as_address(), market.market)
                .call()
                .await
                .map_err(|_| BorrowingProviderError::Unavailable)?;
            if enabled {
                enabled_collateral.insert(token);
            }
        }

        let mut weighted_collateral_usd = Decimal::ZERO;
        let mut borrowed_usd = Decimal::ZERO;
        for position in positions {
            for asset in &position.supplied {
                if enabled_collateral.contains(&asset.token.contract) {
                    let threshold = markets
                        .get(&asset.token.contract)
                        .ok_or(BorrowingProviderError::InvalidData)?
                        .liquidation_threshold;
                    weighted_collateral_usd = checked_add(
                        weighted_collateral_usd,
                        checked_mul(asset.value_usd, threshold)?,
                    )?;
                }
            }
            for asset in &position.borrowed {
                borrowed_usd = checked_add(borrowed_usd, asset.value_usd)?;
            }
        }

        let added_debt_usd = checked_mul(borrow_amount, debt_market.underlying_price_usd)?;
        let projected_debt_usd = checked_add(borrowed_usd, added_debt_usd)?;

        preview.health_factor_before = health_factor(weighted_collateral_usd, borrowed_usd)?;
        preview.health_factor_after = health_factor(weighted_collateral_usd, projected_debt_usd)?;
        Ok(())
    }
}

fn health_factor(
    weighted_collateral_usd: Decimal,
    borrowed_usd: Decimal,
) -> Result<Option<Decimal>, BorrowingProviderError> {
    if borrowed_usd.is_zero() {
        return Ok(None);
    }
    weighted_collateral_usd
        .checked_div(borrowed_usd)
        .map(Some)
        .ok_or(BorrowingProviderError::InvalidData)
}

fn checked_add(left: Decimal, right: Decimal) -> Result<Decimal, BorrowingProviderError> {
    left.checked_add(right)
        .ok_or(BorrowingProviderError::InvalidData)
}

fn checked_mul(left: Decimal, right: Decimal) -> Result<Decimal, BorrowingProviderError> {
    left.checked_mul(right)
        .ok_or(BorrowingProviderError::InvalidData)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_factor_uses_liquidation_weighted_collateral() {
        assert_eq!(
            health_factor(Decimal::new(2_604_417_641, 9), Decimal::new(1, 2)),
            Ok(Some(Decimal::new(2_604_417_641, 7)))
        );
        assert_eq!(
            health_factor(Decimal::new(2_604_417_641, 9), Decimal::ZERO),
            Ok(None)
        );
    }
}
