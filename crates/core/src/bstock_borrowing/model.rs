use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::serialization::serialize_as_string;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BorrowingMarketQuery {
    pub collateral_token: Option<Address>,
    pub debt_token: Option<Address>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingVenue {
    Venus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingToken {
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
    pub symbol: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingMarketUnavailableReason {
    MarketUnlisted,
    SupplyPaused,
    SupplyCapReached,
    BorrowPaused,
    BorrowCapReached,
    InsufficientBorrowLiquidity,
    InvalidOraclePrice,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingMarket {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub collateral: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    pub collateral_market: Address,
    pub debt: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    pub debt_market: Address,
    pub collateral_factor: Decimal,
    pub liquidation_threshold: Decimal,
    pub remaining_supply_capacity: Decimal,
    pub available_borrow_liquidity: Decimal,
    pub borrow_apy_percent: Decimal,
    pub supply_available: bool,
    pub borrow_available: bool,
    pub oracle_valid: bool,
    pub unavailable_reasons: Vec<BorrowingMarketUnavailableReason>,
    pub observed_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingAsset {
    pub token: BorrowingToken,
    pub amount: Decimal,
    pub value_usd: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingPosition {
    #[serde(serialize_with = "serialize_as_string")]
    pub pool: Address,
    pub supplied: Vec<BorrowingAsset>,
    pub borrowed: Vec<BorrowingAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingAccount {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub positions: Vec<BorrowingPosition>,
    pub total_supplied_usd: Decimal,
    pub total_borrowed_usd: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor: Option<Decimal>,
    pub observed_at_ms: i64,
}
