use std::collections::{HashMap, HashSet};

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingMarket, BorrowingMarketUnavailableReason, BorrowingReadError,
        BorrowingReader, BorrowingToken, BorrowingVenue,
    },
    bstock_market::{BNB_CHAIN_ID, BstockId},
    wallet_portfolio::WalletAddress,
};
use venus_api::{
    apis::markets_api::{GetMarketsParams, get_markets},
    models::Market,
};

use super::client::VenusBorrowingReader;

const PAGE_SIZE: i32 = 100;
const WAD: u64 = 1_000_000_000_000_000_000;
const MINT_ACTION: u32 = 0;
const BORROW_ACTION: u32 = 2;
const LISTED: u8 = 1 << 0;
const BORROWABLE: u8 = 1 << 1;
const COLLATERAL_ELIGIBLE: u8 = 1 << 2;
const ORACLE_VALID: u8 = 1 << 3;

struct MarketState {
    market: Address,
    comptroller: Address,
    token: BorrowingToken,
    token_decimals: u32,
    borrow_apy_percent: Decimal,
    collateral_factor: U256,
    liquidation_threshold: U256,
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
    type Error = BorrowingReadError;

    fn try_from(value: Market) -> Result<Self, Self::Error> {
        let token_decimals = u32::try_from(required(value.underlying_decimal)?)
            .map_err(|_| BorrowingReadError::InvalidData)?;
        let underlying_price = parse_u256(&required_nested(value.underlying_price_mantissa)?)?;
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
            market: parse_address(&value.address)?,
            comptroller: parse_address(&value.pool_comptroller_address)?,
            token: BorrowingToken {
                contract: parse_address(&required(value.underlying_address)?)?,
                symbol: required(value.underlying_symbol)?,
            },
            token_decimals,
            borrow_apy_percent: parse_decimal(&required(value.borrow_apy)?)?,
            collateral_factor: parse_u256(&required_nested(value.collateral_factor_mantissa)?)?,
            liquidation_threshold: parse_u256(&required_nested(
                value.liquidation_threshold_mantissa,
            )?)?,
            supply_cap: parse_u256(&required_nested(value.supply_caps_mantissa)?)?,
            borrow_cap: parse_u256(&required_nested(value.borrow_caps_mantissa)?)?,
            total_supply: parse_u256(&required_nested(value.total_supply_mantissa)?)?,
            exchange_rate: parse_u256(&required_nested(value.exchange_rate_mantissa)?)?,
            total_borrows: parse_u256(&required_nested(value.total_borrows_mantissa)?)?,
            cash: parse_u256(&required_nested(value.cash_mantissa)?)?,
            conditions,
            paused_actions: u32::try_from(required_nested(value.paused_actions_bitmap)?)
                .map_err(|_| BorrowingReadError::InvalidData)?,
        })
    }
}

impl MarketState {
    fn has(&self, condition: u8) -> bool {
        self.conditions & condition != 0
    }
}

#[async_trait]
impl BorrowingReader for VenusBorrowingReader {
    async fn read_markets(
        &self,
        collateral_assets: &[BstockId],
        debt_tokens: &[Address],
    ) -> Result<Vec<BorrowingMarket>, BorrowingReadError> {
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
    ) -> Result<BorrowingAccount, BorrowingReadError> {
        self.read_binance_account(wallet).await
    }
}

impl VenusBorrowingReader {
    async fn market_states(
        &self,
        requested_tokens: &HashSet<Address>,
    ) -> Result<HashMap<Address, MarketState>, BorrowingReadError> {
        let mut page = 0;
        let mut fetched = 0;
        let mut states = HashMap::new();

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
            .map_err(|_| BorrowingReadError::Unavailable)?;
            let total =
                usize::try_from(response.total).map_err(|_| BorrowingReadError::InvalidData)?;
            let result_count = response.result.len();
            fetched += result_count;

            for market in response.result {
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

                let state = MarketState::try_from(market)?;
                if state.comptroller == self.comptroller {
                    states.insert(contract, state);
                }
            }

            if result_count == 0 || fetched >= total {
                break;
            }
            page += 1;
        }

        Ok(states)
    }
}

fn build_market(
    collateral: &MarketState,
    debt: &MarketState,
    observed_at_ms: i64,
) -> Result<BorrowingMarket, BorrowingReadError> {
    let supplied = collateral
        .total_supply
        .checked_mul(collateral.exchange_rate)
        .and_then(|value| value.checked_div(U256::from(WAD)))
        .ok_or(BorrowingReadError::InvalidData)?;
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
        collateral_factor: decimal_from_u256(collateral.collateral_factor, 18)?,
        liquidation_threshold: decimal_from_u256(collateral.liquidation_threshold, 18)?,
        remaining_supply_capacity: decimal_from_u256(
            remaining_supply_capacity,
            collateral.token_decimals,
        )?,
        available_borrow_liquidity: decimal_from_u256(
            available_borrow_liquidity,
            debt.token_decimals,
        )?,
        borrow_apy_percent: debt.borrow_apy_percent,
        supply_available,
        borrow_available,
        oracle_valid,
        unavailable_reasons,
        observed_at_ms,
    })
}

fn action_paused(bitmap: u32, action: u32) -> bool {
    bitmap & (1 << action) != 0
}

fn parse_address(value: &str) -> Result<Address, BorrowingReadError> {
    value.parse().map_err(|_| BorrowingReadError::InvalidData)
}

fn parse_u256(value: &str) -> Result<U256, BorrowingReadError> {
    value.parse().map_err(|_| BorrowingReadError::InvalidData)
}

fn parse_decimal(value: &str) -> Result<Decimal, BorrowingReadError> {
    value.parse().map_err(|_| BorrowingReadError::InvalidData)
}

fn decimal_from_u256(value: U256, decimals: u32) -> Result<Decimal, BorrowingReadError> {
    let scale = 10_u64
        .checked_pow(decimals)
        .map(Decimal::from)
        .ok_or(BorrowingReadError::InvalidData)?;
    parse_decimal(&value.to_string())?
        .checked_div(scale)
        .ok_or(BorrowingReadError::InvalidData)
}

fn required<T>(value: Option<T>) -> Result<T, BorrowingReadError> {
    value.ok_or(BorrowingReadError::InvalidData)
}

#[allow(clippy::option_option)]
fn required_nested<T>(value: Option<Option<T>>) -> Result<T, BorrowingReadError> {
    value.flatten().ok_or(BorrowingReadError::InvalidData)
}
