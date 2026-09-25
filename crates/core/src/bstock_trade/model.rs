use alloy_primitives::{Address, B256, Bytes, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use uuid::Uuid;

use crate::wallet_portfolio::WalletAddress;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RfqVenue {
    #[serde(alias = "InchFusion")]
    InchFusion,
    #[serde(alias = "CowSwap")]
    CowSwap,
    #[serde(alias = "PcsXRfq")]
    PcsXRfq,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_address")]
    pub bstock_contract: Address,
    #[serde(deserialize_with = "deserialize_address")]
    pub settlement_token: Address,
    #[serde(deserialize_with = "deserialize_u256")]
    pub raw_settlement_amount: U256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    #[serde(deserialize_with = "deserialize_address")]
    pub settlement_token: Address,
    #[serde(deserialize_with = "deserialize_u256")]
    pub raw_settlement_amount: U256,
    pub venue: RfqVenue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareTradeRequest {
    #[serde(flatten)]
    pub trade: TradeRequest,
    pub quote_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTradeRequest {
    pub request_id: Uuid,
    pub quote_id: String,
    pub venue: RfqVenue,
    pub user_signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuote {
    pub quote_id: String,
    pub venue: RfqVenue,
    #[serde(serialize_with = "serialize_u256")]
    pub raw_bstock_amount: U256,
    pub price_impact_percent: Decimal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedApproval {
    #[serde(serialize_with = "serialize_address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_bytes")]
    pub calldata: Bytes,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_limit: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_price: U256,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTrade {
    pub request_id: Uuid,
    pub quote_id: String,
    pub venue: RfqVenue,
    pub typed_data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TradeState {
    PendingVendor,
    PendingOnchain,
    Filled,
    Failed,
    Expired,
    Cancelled,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedTrade {
    pub order_id: String,
    pub state: TradeState,
    pub created_at_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeStatus {
    pub state: TradeState,
    #[serde(serialize_with = "serialize_optional_hash")]
    pub transaction_hash: Option<B256>,
    #[serde(serialize_with = "serialize_optional_u256")]
    pub raw_settlement_amount: Option<U256>,
    #[serde(serialize_with = "serialize_optional_u256")]
    pub raw_bstock_amount: Option<U256>,
    pub created_at_ms: i64,
    pub filled_at_ms: Option<i64>,
}

fn serialize_address<S>(value: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(value)
}

#[allow(clippy::ref_option)]
fn serialize_optional_u256<S>(value: &Option<U256>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}

#[allow(clippy::ref_option)]
fn serialize_optional_hash<S>(value: &Option<B256>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}

fn serialize_bytes<S>(value: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{value:#x}"))
}

fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
}

fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
}
