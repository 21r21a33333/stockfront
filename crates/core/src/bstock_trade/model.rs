use alloy_primitives::{Address, B256, Bytes, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    serialization::{
        deserialize_from_string, serialize_as_string, serialize_bytes, serialize_optional_as_string,
    },
    wallet_portfolio::WalletAddress,
};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RfqVenue {
    #[serde(alias = "InchFusion")]
    InchFusion,
    #[serde(alias = "CowSwap")]
    CowSwap,
    #[serde(alias = "PcsXRfq")]
    PcsXRfq,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeExecutionMode {
    Swap,
    Rfq,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuoteRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub sell_token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub buy_token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub sell_amount: U256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
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

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitOrderRequest {
    pub submission_id: Uuid,
    pub route_quote_id: String,
    pub venue: RfqVenue,
    pub signature: String,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    #[serde(
        deserialize_with = "deserialize_from_string",
        serialize_with = "serialize_as_string"
    )]
    #[schema(value_type = String, format = "evm-address")]
    pub token: Address,
    #[serde(
        deserialize_with = "deserialize_from_string",
        serialize_with = "serialize_as_string"
    )]
    #[schema(value_type = String)]
    pub amount: U256,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettledTokenAmount {
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub token: Address,
    #[schema(value_type = String)]
    pub amount: Decimal,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuote {
    pub route_quote_id: String,
    pub venue: String,
    pub execution_mode: TradeExecutionMode,
    pub sell: TokenAmount,
    pub buy: TokenAmount,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub price_impact_percent: Option<Decimal>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, format = "evm-address", nullable = false)]
    pub approval_target: Option<Address>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedApproval {
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub spender: Address,
    #[serde(serialize_with = "serialize_bytes")]
    #[schema(value_type = String, format = "hex")]
    pub data: Bytes,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub gas_limit: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub gas_price: Option<U256>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
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

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTransaction {
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_bytes")]
    #[schema(value_type = String, format = "hex")]
    pub data: Bytes,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub value: U256,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub gas_limit: U256,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub gas_price: U256,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub min_receive_amount: U256,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
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

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedOrder {
    pub order_id: String,
    pub state: TradeOrderState,
    pub created_at_ms: i64,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeOrderStatus {
    pub state: TradeOrderState,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, format = "hex", nullable = false)]
    pub transaction_hash: Option<B256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub filled_sell_amount: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub filled_buy_amount: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub created_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub filled_at_ms: Option<i64>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TradeTransactionState {
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeTransactionStatus {
    pub state: TradeTransactionState,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sold: Vec<SettledTokenAmount>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bought: Vec<SettledTokenAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub block_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub executed_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub error: Option<String>,
}

/// Configured settlement-token metadata read from the token contracts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementTokenMetadata {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
}
