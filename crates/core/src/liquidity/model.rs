use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{bstock_borrowing::BorrowingToken, carry::EarnRateType};

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
