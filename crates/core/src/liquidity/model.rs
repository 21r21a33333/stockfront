use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    bstock_borrowing::{BorrowingAsset, BorrowingToken},
    carry::EarnRateType,
    prepared_action::{ActionPreview, PreparedTransaction},
    serialization::deserialize_from_string,
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityOpportunity {
    pub investment_id: String,
    pub investment_name: String,
    pub protocol_id: String,
    pub protocol_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_logo_url: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::serialization::serialize_optional_as_string"
    )]
    pub pool_address: Option<Address>,
    pub assets: Vec<BorrowingToken>,
    pub rate_type: EarnRateType,
    pub reported_rate_bps: i64,
    pub reported_rate_display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_tvl_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_rate: Option<Decimal>,
    pub investable: bool,
    pub observed_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidityClaimAction {
    LpFee,
    RewardInvestment,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPosition {
    pub position_id: String,
    pub investment_ids: Vec<String>,
    pub protocol_id: String,
    pub protocol_name: String,
    #[serde(serialize_with = "crate::serialization::serialize_as_string")]
    pub pool_address: Address,
    pub nft_id: String,
    pub value_usd: Decimal,
    pub assets: Vec<BorrowingAsset>,
    pub claimable_assets: Vec<BorrowingAsset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_tier: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tick: Option<i32>,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub in_range: bool,
    pub claim_actions: Vec<LiquidityClaimAction>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityAccount {
    pub chain_id: u64,
    pub positions: Vec<LiquidityPosition>,
    pub total_value_usd: Decimal,
    pub observed_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityAmount {
    #[serde(
        deserialize_with = "deserialize_from_string",
        serialize_with = "crate::serialization::serialize_as_string"
    )]
    pub token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareLiquidityEntryRequest {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub input: LiquidityAmount,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub price_range_percent: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiquidityEntryCommand {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub input: LiquidityAmount,
    pub price_range_percent: Decimal,
    pub slippage_bps: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidityTransactionKind {
    Approve,
    LpAdd,
}

pub type PreparedLiquidityTransaction = PreparedTransaction<LiquidityTransactionKind>;

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedLiquidityEntry {
    pub required_amounts: Vec<LiquidityAmount>,
    pub transactions: Vec<PreparedLiquidityTransaction>,
    pub preview: ActionPreview,
}
