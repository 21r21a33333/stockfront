use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::bstock_borrowing::BorrowingToken;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CarryOpportunityQuery {
    pub collateral_token: Address,
    pub debt_token: Address,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryQualification {
    Eligible,
    ReviewRequired,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CarryOpportunityReason {
    EarnUnavailable,
    BorrowUnavailable,
    InsufficientBaseSpread,
    UnreviewedInvestment,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
    pub base_spread_percent: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_tvl_usd: Option<Decimal>,
    pub available_borrow_liquidity: Decimal,
    pub qualification: CarryQualification,
    pub reasons: Vec<CarryOpportunityReason>,
    pub observed_at_ms: i64,
}
