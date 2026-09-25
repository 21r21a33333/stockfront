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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeExecutionMode {
    Swap,
    Rfq,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuoteRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_address")]
    pub sell_token: Address,
    #[serde(deserialize_with = "deserialize_address")]
    pub buy_token: Address,
    #[serde(deserialize_with = "deserialize_u256")]
    pub sell_amount: U256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    #[serde(deserialize_with = "deserialize_address")]
    pub token: Address,
    #[serde(deserialize_with = "deserialize_u256")]
    pub amount: U256,
    pub venue: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareTradeRequest {
    #[serde(flatten)]
    pub trade: TradeQuoteRequest,
    pub route_quote_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitOrderRequest {
    pub submission_id: Uuid,
    pub route_quote_id: String,
    pub venue: RfqVenue,
    pub signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    #[serde(serialize_with = "serialize_address")]
    pub token: Address,
    #[serde(serialize_with = "serialize_u256")]
    pub amount: U256,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettledTokenAmount {
    #[serde(serialize_with = "serialize_address")]
    pub token: Address,
    pub amount: Decimal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuote {
    pub route_quote_id: String,
    pub venue: String,
    pub execution_mode: TradeExecutionMode,
    pub sell: TokenAmount,
    pub buy: TokenAmount,
    pub price_impact_percent: Decimal,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_address"
    )]
    pub approval_target: Option<Address>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedApproval {
    #[serde(serialize_with = "serialize_address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_address")]
    pub spender: Address,
    #[serde(serialize_with = "serialize_bytes")]
    pub data: Bytes,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_limit: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_price: U256,
}

#[derive(Debug, Serialize)]
#[serde(tag = "executionMode", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreparedTrade {
    Swap {
        transaction: PreparedTransaction,
    },
    Rfq {
        #[serde(rename = "submissionId")]
        submission_id: Uuid,
        #[serde(rename = "routeQuoteId")]
        route_quote_id: String,
        venue: RfqVenue,
        #[serde(rename = "typedData")]
        typed_data: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTransaction {
    #[serde(serialize_with = "serialize_address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_bytes")]
    pub data: Bytes,
    #[serde(serialize_with = "serialize_u256")]
    pub value: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_limit: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub gas_price: U256,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_u256"
    )]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(serialize_with = "serialize_u256")]
    pub min_receive_amount: U256,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TradeOrderState {
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
pub struct SubmittedOrder {
    pub order_id: String,
    pub state: TradeOrderState,
    pub created_at_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeOrderStatus {
    pub state: TradeOrderState,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_hash"
    )]
    pub transaction_hash: Option<B256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_u256"
    )]
    pub filled_sell_amount: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_u256"
    )]
    pub filled_buy_amount: Option<U256>,
    pub created_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filled_at_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TradeTransactionState {
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeTransactionStatus {
    pub state: TradeTransactionState,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sold: Vec<SettledTokenAmount>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bought: Vec<SettledTokenAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn serialize_address<S>(value: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[allow(clippy::ref_option)]
fn serialize_optional_address<S>(value: &Option<Address>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
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
