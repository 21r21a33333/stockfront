use alloy_primitives::{Address, Bytes, U256};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::serialization::{serialize_as_string, serialize_bytes, serialize_optional_as_string};

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTransaction<K> {
    pub kind: K,
    #[serde(serialize_with = "serialize_as_string")]
    pub to: Address,
    #[serde(serialize_with = "serialize_bytes")]
    pub data: Bytes,
    #[serde(serialize_with = "serialize_as_string")]
    pub value: U256,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    pub gas_limit: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    pub gas_price: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    pub max_fee_per_gas: Option<U256>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFee {
    pub amount: Decimal,
    pub token_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_usd: Option<Decimal>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionWarning {
    pub code: String,
    pub level: String,
    pub message: String,
}
