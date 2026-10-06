use std::collections::HashSet;

use alloy_primitives::Address;
use rust_decimal::{Decimal, RoundingStrategy};
use stockfront_core::{
    bstock_borrowing::{
        BorrowCapability, BorrowCapabilityRequest, BorrowCapabilityUnavailableReason, BorrowStage,
        BorrowingPreview, BorrowingProviderError, BorrowingVenue,
    },
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::{
    client::VenusBorrowingAdapter, contracts::IVenusComptroller, conversion::decode_units,
    markets::MarketState,
};

struct AccountRisk {
    current_weighted_collateral_usd: Decimal,
    projected_weighted_collateral_usd: Decimal,
    borrowed_usd: Decimal,
    selected_position_found: bool,
    selected_collateral_enabled: bool,
}

impl VenusBorrowingAdapter {
    pub(super) async fn apply_borrow_risk(
        &self,
        wallet: WalletAddress,
        debt_market: &MarketState,
        borrow_amount: Decimal,
        preview: &mut BorrowingPreview,
    ) -> Result<(), BorrowingProviderError> {
        let risk = self.account_risk(wallet, None).await?;
        let added_debt_usd = checked_mul(borrow_amount, debt_market.underlying_price_usd)?;
        let projected_debt_usd = checked_add(risk.borrowed_usd, added_debt_usd)?;

        preview.health_factor_before =
            health_factor(risk.current_weighted_collateral_usd, risk.borrowed_usd)?;
        preview.health_factor_after =
            health_factor(risk.current_weighted_collateral_usd, projected_debt_usd)?;
        Ok(())
    }

    pub(super) async fn borrow_capability(
        &self,
        request: &BorrowCapabilityRequest,
    ) -> Result<BorrowCapability, BorrowingProviderError> {
        let requested_tokens = HashSet::from([request.collateral_token, request.debt_token]);
        let states = self.market_states(&requested_tokens).await?;
        let collateral = states
            .get(&request.collateral_token)
            .ok_or(BorrowingProviderError::Rejected)?;
        let debt = states
            .get(&request.debt_token)
            .ok_or(BorrowingProviderError::Rejected)?;
        let requested_amount = decode_units(request.raw_amount, debt.token_decimals)?;
        let liquidity = decode_units(debt.available_borrow_liquidity(), debt.token_decimals)?;
        let risk = self
            .account_risk(request.wallet, Some(request.collateral_token))
            .await?;
        let mut reasons = Vec::new();

        if request.pool != self.comptroller {
            reasons.push(BorrowCapabilityUnavailableReason::UnsupportedPool);
        }
        if !risk.selected_position_found {
            reasons.push(BorrowCapabilityUnavailableReason::PositionNotFound);
        }
        if !collateral.is_listed() || !debt.is_listed() {
            reasons.push(BorrowCapabilityUnavailableReason::MarketUnlisted);
        }
        if !collateral.accepts_collateral() {
            reasons.push(BorrowCapabilityUnavailableReason::CollateralUnavailable);
        }
        if debt.borrow_paused() || !debt.is_borrowable() {
            reasons.push(BorrowCapabilityUnavailableReason::BorrowPaused);
        }
        if debt.borrow_cap_reached() {
            reasons.push(BorrowCapabilityUnavailableReason::BorrowCapReached);
        }
        if request.raw_amount > debt.available_borrow_liquidity() {
            reasons.push(BorrowCapabilityUnavailableReason::InsufficientBorrowLiquidity);
        }
        if !collateral.oracle_valid() || !debt.oracle_valid() {
            reasons.push(BorrowCapabilityUnavailableReason::InvalidOraclePrice);
        }

        let (conservative_max_amount, health_factor_before, health_factor_after) = if risk
            .selected_position_found
            && collateral.oracle_valid()
            && debt.oracle_valid()
            && debt.underlying_price_usd > Decimal::ZERO
        {
            let before = health_factor(risk.current_weighted_collateral_usd, risk.borrowed_usd)?;
            let projected_debt = checked_add(
                risk.borrowed_usd,
                checked_mul(requested_amount, debt.underlying_price_usd)?,
            )?;
            let after = health_factor(risk.projected_weighted_collateral_usd, projected_debt)?;
            let safe_token_amount = conservative_max_amount(
                risk.projected_weighted_collateral_usd,
                risk.borrowed_usd,
                request.minimum_health_factor,
                debt.underlying_price_usd,
                liquidity,
                debt.token_decimals,
            )?;
            if after.is_none() {
                reasons.push(BorrowCapabilityUnavailableReason::HealthFactorUnavailable);
            } else if after.is_some_and(|value| value < request.minimum_health_factor) {
                reasons.push(BorrowCapabilityUnavailableReason::HealthFactorBelowMinimum);
            }
            (safe_token_amount, before, after)
        } else {
            (Decimal::ZERO, None, None)
        };

        reasons.sort_unstable_by_key(|reason| *reason as u8);
        reasons.dedup();
        Ok(BorrowCapability {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            pool: request.pool,
            collateral: collateral.token.clone(),
            debt: debt.token.clone(),
            requested_amount,
            requested_raw_amount: request.raw_amount,
            debt_token_decimals: debt.token_decimals,
            conservative_max_amount,
            available_borrow_liquidity: liquidity,
            borrow_apy_percent: debt.borrow_apy_percent,
            health_factor_before,
            health_factor_after,
            collateral_enabled: risk.selected_collateral_enabled,
            stage: if risk.selected_collateral_enabled {
                BorrowStage::Borrow
            } else {
                BorrowStage::EnableCollateral
            },
            unavailable_reasons: reasons,
            observed_at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn account_risk(
        &self,
        wallet: WalletAddress,
        selected_collateral: Option<Address>,
    ) -> Result<AccountRisk, BorrowingProviderError> {
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
        for token in &supplied_tokens {
            let market = markets
                .get(token)
                .ok_or(BorrowingProviderError::InvalidData)?;
            if comptroller
                .checkMembership(wallet.as_address(), market.market)
                .call()
                .await
                .map_err(|_| BorrowingProviderError::Unavailable)?
            {
                enabled_collateral.insert(*token);
            }
        }

        let selected_position_found =
            selected_collateral.is_none_or(|token| supplied_tokens.contains(&token));
        let selected_collateral_enabled =
            selected_collateral.is_some_and(|token| enabled_collateral.contains(&token));
        let mut current_weighted_collateral_usd = Decimal::ZERO;
        let mut projected_weighted_collateral_usd = Decimal::ZERO;
        let mut borrowed_usd = Decimal::ZERO;
        for position in positions {
            for asset in &position.supplied {
                let market = markets
                    .get(&asset.token.contract)
                    .ok_or(BorrowingProviderError::InvalidData)?;
                let weighted = checked_mul(asset.value_usd, market.liquidation_threshold)?;
                if enabled_collateral.contains(&asset.token.contract) {
                    current_weighted_collateral_usd =
                        checked_add(current_weighted_collateral_usd, weighted)?;
                }
                if enabled_collateral.contains(&asset.token.contract)
                    || selected_collateral == Some(asset.token.contract)
                {
                    projected_weighted_collateral_usd =
                        checked_add(projected_weighted_collateral_usd, weighted)?;
                }
            }
            for asset in &position.borrowed {
                borrowed_usd = checked_add(borrowed_usd, asset.value_usd)?;
            }
        }

        Ok(AccountRisk {
            current_weighted_collateral_usd,
            projected_weighted_collateral_usd,
            borrowed_usd,
            selected_position_found,
            selected_collateral_enabled,
        })
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

fn conservative_max_amount(
    weighted_collateral_usd: Decimal,
    borrowed_usd: Decimal,
    minimum_health_factor: Decimal,
    debt_price_usd: Decimal,
    liquidity: Decimal,
    token_decimals: u32,
) -> Result<Decimal, BorrowingProviderError> {
    if minimum_health_factor <= Decimal::ZERO || debt_price_usd <= Decimal::ZERO {
        return Err(BorrowingProviderError::InvalidData);
    }
    let safe_debt_usd = weighted_collateral_usd
        .checked_div(minimum_health_factor)
        .and_then(|limit| limit.checked_sub(borrowed_usd))
        .unwrap_or(Decimal::ZERO)
        .max(Decimal::ZERO);
    safe_debt_usd
        .checked_div(debt_price_usd)
        .map(|amount| {
            amount
                .min(liquidity)
                .round_dp_with_strategy(token_decimals, RoundingStrategy::ToZero)
        })
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

    #[test]
    fn conservative_max_respects_health_factor_and_liquidity() {
        assert_eq!(
            conservative_max_amount(
                Decimal::new(150, 0),
                Decimal::new(40, 0),
                Decimal::new(15, 1),
                Decimal::ONE,
                Decimal::new(100, 0),
                18,
            ),
            Ok(Decimal::new(60, 0))
        );
        assert_eq!(
            conservative_max_amount(
                Decimal::new(150, 0),
                Decimal::ZERO,
                Decimal::new(15, 1),
                Decimal::ONE,
                Decimal::new(25, 0),
                18,
            ),
            Ok(Decimal::new(25, 0))
        );
    }
}
