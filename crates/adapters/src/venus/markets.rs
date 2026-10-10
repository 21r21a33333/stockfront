use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{
        BorrowCapability, BorrowCapabilityRequest, BorrowingAccount, BorrowingMarket,
        BorrowingMarketUnavailableReason, BorrowingProviderError, BorrowingReader, BorrowingToken,
        BorrowingVenue, RepaymentCapability, RepaymentCapabilityRequest, SupplyMarket,
        SupplyMarketUnavailableReason,
    },
    bstock_market::{BNB_CHAIN_ID, BstockId},
    wallet_portfolio::WalletAddress,
};
use venus_api::{
    apis::markets_api::{GetMarketsParams, get_markets},
    models::Market,
};

use super::{
    client::VenusBorrowingAdapter,
    conversion::{decode_units, parse, required, required_nested},
};

const PAGE_SIZE: i32 = 100;
const WAD: u64 = 1_000_000_000_000_000_000;
const MINT_ACTION: u32 = 0;
const BORROW_ACTION: u32 = 2;
const LISTED: u8 = 1 << 0;
const BORROWABLE: u8 = 1 << 1;
const COLLATERAL_ELIGIBLE: u8 = 1 << 2;
const ORACLE_VALID: u8 = 1 << 3;

pub(super) struct MarketState {
    pub(super) market: Address,
    comptroller: Address,
    pub(super) token: BorrowingToken,
    pub(super) token_decimals: u32,
    pub(super) underlying_price_usd: Decimal,
    pub(super) borrow_apy_percent: Decimal,
    supply_apy_percent: Decimal,
    collateral_factor: Decimal,
    pub(super) liquidation_threshold: Decimal,
    supply_cap: U256,
    borrow_cap: U256,
    total_supply: U256,
    exchange_rate: U256,
    total_borrows: U256,
    cash: U256,
    conditions: u8,
    paused_actions: u32,
}

impl TryFrom<Market> for MarketState {
    type Error = BorrowingProviderError;

    fn try_from(value: Market) -> Result<Self, Self::Error> {
        let token_decimals = u32::try_from(required(value.underlying_decimal)?)
            .map_err(|_| BorrowingProviderError::InvalidData)?;
        let underlying_price: U256 = parse(&required_nested(value.underlying_price_mantissa)?)?;
        let conditions = [
            (required_nested(value.is_listed)?, LISTED),
            (required_nested(value.is_borrowable)?, BORROWABLE),
            (
                required_nested(value.can_be_collateral)?,
                COLLATERAL_ELIGIBLE,
            ),
            (
                !required_nested(value.is_price_invalid)? && underlying_price != U256::ZERO,
                ORACLE_VALID,
            ),
        ]
        .into_iter()
        .fold(
            0,
            |flags, (enabled, flag)| {
                if enabled { flags | flag } else { flags }
            },
        );

        Ok(Self {
            market: parse(&value.address)?,
            comptroller: parse(&value.pool_comptroller_address)?,
            token: BorrowingToken {
                contract: parse(&required(value.underlying_address)?)?,
                symbol: required(value.underlying_symbol)?,
            },
            token_decimals,
            underlying_price_usd: decode_units(underlying_price, 18)?,
            borrow_apy_percent: parse(&required(value.borrow_apy)?)?,
            supply_apy_percent: parse(&required(value.supply_apy)?)?,
            collateral_factor: decode_units(
                parse(&required_nested(value.collateral_factor_mantissa)?)?,
                18,
            )?,
            liquidation_threshold: decode_units(
                parse(&required_nested(value.liquidation_threshold_mantissa)?)?,
                18,
            )?,
            supply_cap: parse(&required_nested(value.supply_caps_mantissa)?)?,
            borrow_cap: parse(&required_nested(value.borrow_caps_mantissa)?)?,
            total_supply: parse(&required_nested(value.total_supply_mantissa)?)?,
            exchange_rate: parse(&required_nested(value.exchange_rate_mantissa)?)?,
            total_borrows: parse(&required_nested(value.total_borrows_mantissa)?)?,
            cash: parse(&required_nested(value.cash_mantissa)?)?,
            conditions,
            paused_actions: u32::try_from(required_nested(value.paused_actions_bitmap)?)
                .map_err(|_| BorrowingProviderError::InvalidData)?,
        })
    }
}

impl MarketState {
    fn has(&self, condition: u8) -> bool {
        self.conditions & condition != 0
    }

    pub(super) fn accepts_collateral(&self) -> bool {
        self.has(LISTED) && self.has(COLLATERAL_ELIGIBLE) && self.has(ORACLE_VALID)
    }

    pub(super) fn accepts_borrow(&self) -> bool {
        self.has(LISTED)
            && self.has(BORROWABLE)
            && self.has(ORACLE_VALID)
            && !action_paused(self.paused_actions, BORROW_ACTION)
    }

    pub(super) fn available_borrow_liquidity(&self) -> U256 {
        self.cash
            .min(self.borrow_cap.saturating_sub(self.total_borrows))
    }

    pub(super) fn is_listed(&self) -> bool {
        self.has(LISTED)
    }

    pub(super) fn is_borrowable(&self) -> bool {
        self.has(BORROWABLE)
    }

    pub(super) fn oracle_valid(&self) -> bool {
        self.has(ORACLE_VALID)
    }

    pub(super) fn borrow_paused(&self) -> bool {
        action_paused(self.paused_actions, BORROW_ACTION)
    }

    pub(super) fn borrow_cap_reached(&self) -> bool {
        self.borrow_cap.saturating_sub(self.total_borrows) == U256::ZERO
    }
}

#[async_trait]
impl BorrowingReader for VenusBorrowingAdapter {
    async fn read_borrow_capability(
        &self,
        request: &BorrowCapabilityRequest,
    ) -> Result<BorrowCapability, BorrowingProviderError> {
        self.borrow_capability(request).await
    }

    async fn read_repayment_capability(
        &self,
        request: &RepaymentCapabilityRequest,
    ) -> Result<RepaymentCapability, BorrowingProviderError> {
        self.repayment_capability(request).await
    }

    async fn read_supply_market(
        &self,
        token: Address,
    ) -> Result<Option<SupplyMarket>, BorrowingProviderError> {
        let state = self
            .market_states(&HashSet::from([token]))
            .await?
            .remove(&token);
        state
            .as_ref()
            .map(|state| build_supply_market(state, chrono::Utc::now().timestamp_millis()))
            .transpose()
    }

    async fn read_markets(
        &self,
        collateral_assets: &[BstockId],
        debt_tokens: &[Address],
    ) -> Result<Vec<BorrowingMarket>, BorrowingProviderError> {
        let requested_tokens = collateral_assets
            .iter()
            .map(|asset| asset.contract)
            .chain(debt_tokens.iter().copied())
            .collect::<HashSet<_>>();
        let states = self.market_states(&requested_tokens).await?;
        let observed_at_ms = chrono::Utc::now().timestamp_millis();
        let mut markets = Vec::new();

        for collateral in collateral_assets
            .iter()
            .filter_map(|asset| states.get(&asset.contract))
        {
            for debt in debt_tokens.iter().filter_map(|token| states.get(token)) {
                markets.push(build_market(collateral, debt, observed_at_ms)?);
            }
        }

        markets.sort_by(|left, right| {
            left.collateral
                .symbol
                .cmp(&right.collateral.symbol)
                .then_with(|| left.debt.symbol.cmp(&right.debt.symbol))
        });
        Ok(markets)
    }

    async fn read_account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingProviderError> {
        self.read_binance_account(wallet).await
    }
}

impl VenusBorrowingAdapter {
    pub(super) async fn market_state(
        &self,
        token: Address,
    ) -> Result<MarketState, BorrowingProviderError> {
        self.market_states(&HashSet::from([token]))
            .await?
            .remove(&token)
            .ok_or(BorrowingProviderError::Rejected)
    }

    /// Returns the live state of each requested token's Venus market.
    ///
    /// The provider only lists every market on the chain, so the listing is read
    /// once per cache window and the requested rows are selected from it here.
    /// That turns the second and later lookups in a request — a borrow reads the
    /// market, then reads it again to price the account's risk — into memory work.
    pub(super) async fn market_states(
        &self,
        requested_tokens: &HashSet<Address>,
    ) -> Result<HashMap<Address, MarketState>, BorrowingProviderError> {
        let markets = self.listed_markets().await?;
        let mut states = HashMap::with_capacity(requested_tokens.len());

        for market in markets.iter() {
            let Some(contract) = market
                .underlying_address
                .as_deref()
                .and_then(|address| address.parse::<Address>().ok())
            else {
                continue;
            };
            if !requested_tokens.contains(&contract) {
                continue;
            }

            let state = MarketState::try_from(market.clone())?;
            if state.comptroller == self.comptroller {
                states.insert(contract, state);
            }
        }

        Ok(states)
    }

    async fn listed_markets(&self) -> Result<Arc<Vec<Market>>, BorrowingProviderError> {
        self.markets
            .get_or_load((), async { self.read_markets().await.map(Arc::new) })
            .await
    }

    async fn read_markets(&self) -> Result<Vec<Market>, BorrowingProviderError> {
        let mut page = 0;
        let mut markets = Vec::new();

        loop {
            let response = get_markets(
                &self.api,
                GetMarketsParams {
                    accept_version: "stable".to_owned(),
                    chain_id: BNB_CHAIN_ID.to_string(),
                    limit: Some(PAGE_SIZE),
                    page: Some(page),
                },
            )
            .await
            .map_err(|_| BorrowingProviderError::Unavailable)?;
            let total =
                usize::try_from(response.total).map_err(|_| BorrowingProviderError::InvalidData)?;
            let result_count = response.result.len();
            markets.extend(response.result);

            if result_count == 0 || markets.len() >= total {
                break;
            }
            page += 1;
        }

        Ok(markets)
    }
}

fn build_market(
    collateral: &MarketState,
    debt: &MarketState,
    observed_at_ms: i64,
) -> Result<BorrowingMarket, BorrowingProviderError> {
    let supplied = collateral
        .total_supply
        .checked_mul(collateral.exchange_rate)
        .and_then(|value| value.checked_div(U256::from(WAD)))
        .ok_or(BorrowingProviderError::InvalidData)?;
    let remaining_supply_capacity = collateral.supply_cap.saturating_sub(supplied);
    let remaining_borrow_capacity = debt.borrow_cap.saturating_sub(debt.total_borrows);
    let available_borrow_liquidity = debt.cash.min(remaining_borrow_capacity);
    let supply_cap_reached = remaining_supply_capacity == U256::ZERO;
    let borrow_cap_reached = remaining_borrow_capacity == U256::ZERO;
    let supply_paused = action_paused(collateral.paused_actions, MINT_ACTION);
    let borrow_paused = action_paused(debt.paused_actions, BORROW_ACTION);
    let oracle_valid = collateral.has(ORACLE_VALID) && debt.has(ORACLE_VALID);
    let supply_available = collateral.has(LISTED)
        && collateral.has(COLLATERAL_ELIGIBLE)
        && !supply_paused
        && !supply_cap_reached
        && collateral.has(ORACLE_VALID);
    let borrow_available = debt.has(LISTED)
        && debt.has(BORROWABLE)
        && !borrow_paused
        && !borrow_cap_reached
        && debt.cash != U256::ZERO
        && debt.has(ORACLE_VALID);
    let mut unavailable_reasons = Vec::new();

    if !collateral.has(LISTED) || !debt.has(LISTED) {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::MarketUnlisted);
    }
    if supply_paused {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::SupplyPaused);
    }
    if supply_cap_reached {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::SupplyCapReached);
    }
    if borrow_paused || !debt.has(BORROWABLE) {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::BorrowPaused);
    }
    if borrow_cap_reached {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::BorrowCapReached);
    }
    if debt.cash == U256::ZERO || borrow_cap_reached {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::InsufficientBorrowLiquidity);
    }
    if !oracle_valid {
        unavailable_reasons.push(BorrowingMarketUnavailableReason::InvalidOraclePrice);
    }

    Ok(BorrowingMarket {
        venue: BorrowingVenue::Venus,
        chain_id: BNB_CHAIN_ID,
        collateral: collateral.token.clone(),
        collateral_market: collateral.market,
        debt: debt.token.clone(),
        debt_market: debt.market,
        collateral_factor: collateral.collateral_factor,
        liquidation_threshold: collateral.liquidation_threshold,
        remaining_supply_capacity: decode_units(
            remaining_supply_capacity,
            collateral.token_decimals,
        )?,
        available_borrow_liquidity: decode_units(available_borrow_liquidity, debt.token_decimals)?,
        borrow_apy_percent: debt.borrow_apy_percent,
        supply_available,
        borrow_available,
        oracle_valid,
        unavailable_reasons,
        observed_at_ms,
    })
}

fn build_supply_market(
    state: &MarketState,
    observed_at_ms: i64,
) -> Result<SupplyMarket, BorrowingProviderError> {
    let remaining = if state.supply_cap == U256::MAX {
        None
    } else {
        let supplied = state
            .total_supply
            .checked_mul(state.exchange_rate)
            .and_then(|value| value.checked_div(U256::from(WAD)))
            .ok_or(BorrowingProviderError::InvalidData)?;
        Some(state.supply_cap.saturating_sub(supplied))
    };
    let supply_paused = action_paused(state.paused_actions, MINT_ACTION);
    let supply_cap_reached = remaining == Some(U256::ZERO);
    let mut unavailable_reasons = Vec::new();

    if !state.has(LISTED) {
        unavailable_reasons.push(SupplyMarketUnavailableReason::MarketUnlisted);
    }
    if supply_paused {
        unavailable_reasons.push(SupplyMarketUnavailableReason::SupplyPaused);
    }
    if supply_cap_reached {
        unavailable_reasons.push(SupplyMarketUnavailableReason::SupplyCapReached);
    }

    Ok(SupplyMarket {
        venue: BorrowingVenue::Venus,
        chain_id: BNB_CHAIN_ID,
        token: state.token.clone(),
        market: state.market,
        token_decimals: state.token_decimals,
        supply_apy_percent: state.supply_apy_percent,
        remaining_supply_capacity: remaining
            .map(|value| decode_units(value, state.token_decimals))
            .transpose()?,
        supply_available: state.has(LISTED) && !supply_paused && !supply_cap_reached,
        unavailable_reasons,
        observed_at_ms,
    })
}

fn action_paused(bitmap: u32, action: u32) -> bool {
    bitmap & (1 << action) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_capability_does_not_require_collateral_or_oracle_flags() {
        let market = build_supply_market(&state(LISTED, U256::MAX, 0), 7)
            .unwrap_or_else(|error| panic!("supply market failed: {error}"));

        assert!(market.supply_available);
        assert_eq!(market.remaining_supply_capacity, None);
        assert!(market.unavailable_reasons.is_empty());
    }

    #[test]
    fn zero_cap_and_paused_mint_are_reported_independently() {
        let market = build_supply_market(&state(LISTED, U256::ZERO, 1 << MINT_ACTION), 7)
            .unwrap_or_else(|error| panic!("supply market failed: {error}"));

        assert!(!market.supply_available);
        assert_eq!(market.remaining_supply_capacity, Some(Decimal::ZERO));
        assert_eq!(
            market.unavailable_reasons,
            vec![
                SupplyMarketUnavailableReason::SupplyPaused,
                SupplyMarketUnavailableReason::SupplyCapReached,
            ]
        );
    }

    fn state(conditions: u8, supply_cap: U256, paused_actions: u32) -> MarketState {
        MarketState {
            market: Address::with_last_byte(1),
            comptroller: Address::with_last_byte(2),
            token: BorrowingToken {
                contract: Address::with_last_byte(3),
                symbol: "TSLAB".to_owned(),
            },
            token_decimals: 18,
            underlying_price_usd: Decimal::ONE,
            borrow_apy_percent: Decimal::ZERO,
            supply_apy_percent: Decimal::new(375, 2),
            collateral_factor: Decimal::ZERO,
            liquidation_threshold: Decimal::ZERO,
            supply_cap,
            borrow_cap: U256::ZERO,
            total_supply: U256::ZERO,
            exchange_rate: U256::from(WAD),
            total_borrows: U256::ZERO,
            cash: U256::ZERO,
            conditions,
            paused_actions,
        }
    }
}
