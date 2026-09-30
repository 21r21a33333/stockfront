use alloy_primitives::Address;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

use crate::{bstock_market::BNB_CHAIN_ID, serialization::serialize_as_string};

const DEFAULT_LIST_LIMIT: usize = 50;
const MAX_LIST_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketId {
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
}

impl BasketId {
    #[must_use]
    pub const fn bnb(contract: Address) -> Self {
        Self {
            chain_id: BNB_CHAIN_ID,
            contract,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BasketKind {
    IndexToken,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketAssetAllocation {
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
    pub symbol: String,
    pub name: String,
    pub weight_percent: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketSummary {
    #[serde(flatten)]
    pub id: BasketId,
    pub kind: BasketKind,
    pub provider_id: String,
    pub provider_name: String,
    pub name: String,
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_fee_percent: Option<Decimal>,
    pub assets: Vec<BasketAssetAllocation>,
    pub categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketPosition {
    #[serde(flatten)]
    pub id: BasketId,
    pub kind: BasketKind,
    pub provider_id: String,
    pub provider_name: String,
    pub name: String,
    pub symbol: String,
    pub token_amount: Decimal,
    pub value_usd: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketAccount {
    pub chain_id: u64,
    pub positions: Vec<BasketPosition>,
    pub total_value_usd: Decimal,
    pub observed_at_ms: i64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ListBasketsQuery {
    pub search: Option<String>,
    pub category: Option<String>,
    pub limit: usize,
}

impl ListBasketsQuery {
    /// Builds a normalized, bounded basket query.
    ///
    /// # Errors
    ///
    /// Returns [`ListBasketsQueryError::InvalidLimit`] for zero or excessively large limits.
    pub fn new(
        search: Option<String>,
        category: Option<String>,
        limit: Option<usize>,
    ) -> Result<Self, ListBasketsQueryError> {
        let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT);
        if limit == 0 || limit > MAX_LIST_LIMIT {
            return Err(ListBasketsQueryError::InvalidLimit);
        }

        Ok(Self {
            search: normalize_filter(search),
            category: normalize_filter(category),
            limit,
        })
    }
}

fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ListBasketsQueryError {
    #[error("limit must be between 1 and {MAX_LIST_LIMIT}")]
    InvalidLimit,
}
