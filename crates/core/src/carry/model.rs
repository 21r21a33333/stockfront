use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    bstock_borrowing::{BorrowingAsset, BorrowingToken},
    prepared_action::{ActionPreview, AssetBalanceChange, PreparedTransaction},
    serialization::deserialize_from_string,
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CarryOpportunityQuery {
    pub collateral_token: Address,
    pub debt_token: Address,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EarnRateType {
    Apy,
    Apr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarnOpportunity {
    pub investment_id: String,
    pub investment_name: String,
    pub protocol_id: String,
    pub protocol_name: String,
    pub protocol_logo_url: Option<String>,
    pub asset: BorrowingToken,
    pub rate_type: EarnRateType,
    pub base_rate_bps: i64,
    pub base_rate_display: String,
    pub reported_tvl_usd: Option<Decimal>,
    pub investable: bool,
    pub observed_at_ms: i64,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryPosition {
    pub position_id: String,
    pub investment_ids: Vec<String>,
    pub protocol_id: String,
    pub protocol_name: String,
    pub value_usd: Decimal,
    pub assets: Vec<BorrowingAsset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption: Option<CarryRedemption>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryRedemption {
    pub id: String,
    pub claimable: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryAccount {
    pub chain_id: u64,
    pub positions: Vec<CarryPosition>,
    pub total_value_usd: Decimal,
    pub observed_at_ms: i64,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryQualification {
    Eligible,
    ReviewRequired,
    Unavailable,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryOpportunityReason {
    EarnUnavailable,
    BorrowUnavailable,
    UnsupportedRateType,
    InsufficientBaseSpread,
    UnreviewedInvestment,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryOpportunity {
    pub investment_id: String,
    pub investment_name: String,
    pub protocol_id: String,
    pub protocol_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_logo_url: Option<String>,
    pub asset: BorrowingToken,
    pub rate_type: EarnRateType,
    pub base_earn_rate_bps: i64,
    pub base_earn_rate_display: String,
    pub borrow_apy_percent: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_spread_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_tvl_usd: Option<Decimal>,
    pub available_borrow_liquidity: Decimal,
    pub qualification: CarryQualification,
    pub reasons: Vec<CarryOpportunityReason>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCarryDepositRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_token: Address,
    pub investment_id: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarnDepositCommand {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub asset: Address,
    pub amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "mode", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryWithdrawalAmount {
    Exact {
        #[serde(deserialize_with = "deserialize_from_string")]
        token: Address,
        #[serde(deserialize_with = "deserialize_from_string")]
        value: Decimal,
    },
    Full,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCarryWithdrawalRequest {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub amount: CarryWithdrawalAmount,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarnWithdrawalCommand {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub token: Address,
    pub amount: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCarryRedemptionClaimRequest {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub redemption_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarnRedemptionClaimCommand {
    pub wallet: WalletAddress,
    pub investment_id: String,
    pub redemption_id: String,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryTransactionKind {
    Approve,
    Deposit,
    Redeem,
    Claim,
}

pub type PreparedCarryTransaction = PreparedTransaction<CarryTransactionKind>;

pub type CarryBalanceChange = AssetBalanceChange;

#[derive(utoipa::ToSchema, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryActionPreview {
    #[serde(flatten)]
    pub action: ActionPreview,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_delay: Option<RedemptionDelay>,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryActionPlan {
    // The generic prepared-transaction schema is shared with borrowing, so this
    // one is inlined to keep the carry kinds out of that definition.
    #[schema(inline)]
    pub transactions: Vec<PreparedCarryTransaction>,
    pub preview: CarryActionPreview,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionDelay {
    pub min_days: Decimal,
    pub max_days: Decimal,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedCarryDeposit {
    pub opportunity: CarryOpportunity,
    // The generic prepared-transaction schema is shared with borrowing, so this
    // one is inlined to keep the carry kinds out of that definition.
    #[schema(inline)]
    pub transactions: Vec<PreparedCarryTransaction>,
    pub preview: CarryActionPreview,
}
