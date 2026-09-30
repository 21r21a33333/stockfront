use alloy_primitives::{Address, B256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    baskets::BasketKind, serialization::serialize_as_string, wallet_portfolio::WalletAddress,
};

const DEFAULT_LIST_LIMIT: usize = 20;
const MAX_LIST_LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioCollection {
    Curated,
    TopPerforming,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ManagedPortfolioSort {
    #[serde(rename = "TVL")]
    Tvl,
    #[serde(rename = "INFLOWS_7D")]
    Inflows7d,
    #[serde(rename = "USERS")]
    Users,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListManagedPortfolioTemplatesQuery {
    pub collection: ManagedPortfolioCollection,
    pub sort: Option<ManagedPortfolioSort>,
    pub limit: usize,
    pub cursor: Option<String>,
}

impl ListManagedPortfolioTemplatesQuery {
    /// Builds a bounded provider-page request.
    ///
    /// # Errors
    ///
    /// Returns [`ListManagedPortfolioTemplatesQueryError::InvalidLimit`] when the requested page
    /// size is outside Glider's supported range.
    pub fn new(
        collection: Option<ManagedPortfolioCollection>,
        sort: Option<ManagedPortfolioSort>,
        limit: Option<usize>,
        cursor: Option<String>,
    ) -> Result<Self, ListManagedPortfolioTemplatesQueryError> {
        let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT);
        if limit == 0 || limit > MAX_LIST_LIMIT {
            return Err(ListManagedPortfolioTemplatesQueryError::InvalidLimit);
        }

        Ok(Self {
            collection: collection.unwrap_or(ManagedPortfolioCollection::Curated),
            sort,
            limit,
            cursor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioAllocation {
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
    pub weight_percent: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioPerformance {
    pub window: String,
    pub percent_change: Decimal,
    pub since: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioTemplateMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvl_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_count: Option<u64>,
    pub performance: Vec<ManagedPortfolioPerformance>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioTemplate {
    pub id: String,
    pub kind: BasketKind,
    pub provider_id: String,
    pub provider_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advertised_max_apy_percent: Option<Decimal>,
    pub allocation: Vec<ManagedPortfolioAllocation>,
    pub metrics: ManagedPortfolioTemplateMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioTemplatePage {
    pub items: Vec<ManagedPortfolioTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareManagedPortfolioEnrollmentRequest {
    pub wallet: WalletAddress,
    pub template_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioEnrollmentAuthorization {
    pub flow_id: String,
    pub account_index: String,
    pub agent_account_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedManagedPortfolioEnrollment {
    pub authorization: ManagedPortfolioEnrollmentAuthorization,
    #[serde(serialize_with = "serialize_as_string")]
    pub personal_sign_message: B256,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollManagedPortfolioRequest {
    pub wallet: WalletAddress,
    pub template_id: String,
    pub authorization: ManagedPortfolioEnrollmentAuthorization,
    pub signature: String,
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioAccount {
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    pub address: Address,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioEnrollment {
    pub portfolio_id: String,
    pub template_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub smart_account: ManagedPortfolioAccount,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ListManagedPortfolioTemplatesQueryError {
    #[error("limit must be between 1 and 50")]
    InvalidLimit,
}
