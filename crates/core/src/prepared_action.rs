use alloy_primitives::{Address, Bytes, U256};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{
    bstock_borrowing::BorrowingToken,
    serialization::{serialize_as_string, serialize_bytes, serialize_optional_as_string},
};

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[schema(as = PreparedActionTransaction)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTransaction<K> {
    #[schema(inline)]
    pub kind: K,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub to: Address,
    #[serde(serialize_with = "serialize_bytes")]
    #[schema(value_type = String, format = "hex")]
    pub data: Bytes,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub value: U256,
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub max_priority_fee_per_gas: Option<U256>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_as_string"
    )]
    #[schema(value_type = Option<String>, nullable = false)]
    pub max_fee_per_gas: Option<U256>,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFee {
    pub amount: Decimal,
    pub token_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_usd: Option<Decimal>,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionWarning {
    pub code: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalanceChange {
    pub token: BorrowingToken,
    pub amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_usd: Option<Decimal>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreview {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub balance_changes: Vec<AssetBalanceChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_network_fee: Option<NetworkFee>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::serialization::serialize_optional_as_string"
    )]
    pub interact_with: Option<Address>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ActionWarning>,
}
