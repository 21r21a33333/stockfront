use alloy_primitives::{Address, B256, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    baskets::BasketKind,
    prepared_action::{ActionPreview, PreparedTransaction},
    serialization::{deserialize_from_string, serialize_as_string},
    wallet_portfolio::WalletAddress,
};

const DEFAULT_LIST_LIMIT: usize = 20;
const MAX_LIST_LIMIT: usize = 50;
const MAX_PORTFOLIO_LIST_LIMIT: usize = 200;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListManagedPortfoliosQuery {
    pub limit: usize,
    pub cursor: Option<String>,
}

impl ListManagedPortfoliosQuery {
    /// Builds a bounded provider-page request.
    ///
    /// # Errors
    ///
    /// Returns [`ListManagedPortfoliosQueryError::InvalidLimit`] when the requested page size is
    /// outside Glider's supported range.
    pub fn new(
        limit: Option<usize>,
        cursor: Option<String>,
    ) -> Result<Self, ListManagedPortfoliosQueryError> {
        let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT);
        if limit == 0 || limit > MAX_PORTFOLIO_LIST_LIMIT {
            return Err(ListManagedPortfoliosQueryError::InvalidLimit);
        }

        Ok(Self { limit, cursor })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioAutomationStatus {
    Active,
    Paused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioSchedule {
    pub status: ManagedPortfolioAutomationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<ManagedPortfolioFrequency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_rebalance_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_rebalance_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioSummary {
    pub portfolio_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub template_id: String,
    pub template_name: String,
    pub smart_account: ManagedPortfolioAccount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<ManagedPortfolioSchedule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioPage {
    pub items: Vec<ManagedPortfolioSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioAssetPosition {
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
    pub symbol: String,
    pub decimals: u8,
    #[serde(serialize_with = "serialize_as_string")]
    pub raw_amount: U256,
    pub amount: Decimal,
    pub price_usd: Decimal,
    pub value_usd: Decimal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioWarningKind {
    MissingSmartAccount,
    RpcError,
    MissingPrice,
    BlocklistedAsset,
    MissingAsset,
    InTransitReadFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWarning {
    pub kind: ManagedPortfolioWarningKind,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioPositions {
    pub portfolio_id: String,
    pub smart_account: ManagedPortfolioAccount,
    pub total_value_usd: Decimal,
    pub assets: Vec<ManagedPortfolioAssetPosition>,
    pub warnings: Vec<ManagedPortfolioWarning>,
    pub observed_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrepareManagedPortfolioDepositRequest {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub raw_amount: U256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioTransactionKind {
    Deposit,
}

pub type PreparedManagedPortfolioTransaction = PreparedTransaction<ManagedPortfolioTransactionKind>;

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedManagedPortfolioDeposit {
    pub transactions: Vec<PreparedManagedPortfolioTransaction>,
    pub preview: ActionPreview,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "mode", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrepareManagedPortfolioWithdrawalRequest {
    InKind {
        recipient: WalletAddress,
        assets: Vec<ManagedPortfolioWithdrawalAsset>,
    },
    FullToUsdt {
        recipient: WalletAddress,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalAsset {
    #[serde(
        deserialize_with = "deserialize_from_string",
        serialize_with = "serialize_as_string"
    )]
    pub token: Address,
    #[serde(
        deserialize_with = "deserialize_from_string",
        serialize_with = "serialize_as_string"
    )]
    pub raw_amount: U256,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalEip712Field {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManagedPortfolioWithdrawalEip712Types {
    #[serde(rename = "Withdrawal")]
    pub withdrawal: Vec<ManagedPortfolioWithdrawalEip712Field>,
    #[serde(rename = "WithdrawalAsset")]
    pub withdrawal_asset: Vec<ManagedPortfolioWithdrawalEip712Field>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalEip712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: i32,
    pub verifying_contract: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalMessageAsset {
    pub asset_id: String,
    pub amount_raw: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalMessage {
    pub portfolio_id: String,
    pub recipient_account_id: String,
    pub assets: Vec<ManagedPortfolioWithdrawalMessageAsset>,
    pub nonce: String,
    pub expires_at: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_asset_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioWithdrawalTypedData {
    pub primary_type: String,
    pub domain: ManagedPortfolioWithdrawalEip712Domain,
    pub types: ManagedPortfolioWithdrawalEip712Types,
    pub message: ManagedPortfolioWithdrawalMessage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedManagedPortfolioWithdrawal {
    pub authorization_id: String,
    pub expires_at_ms: i64,
    pub typed_data: ManagedPortfolioWithdrawalTypedData,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitManagedPortfolioWithdrawalRequest {
    pub message: ManagedPortfolioWithdrawalMessage,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedManagedPortfolioWithdrawal {
    pub operation_id: String,
    pub submitted_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManagedPortfolioOperationState {
    Accepted,
    Running,
    Retrying,
    AwaitingUser,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPortfolioOperation {
    pub operation_id: String,
    pub state: ManagedPortfolioOperationState,
    pub updated_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ListManagedPortfolioTemplatesQueryError {
    #[error("limit must be between 1 and 50")]
    InvalidLimit,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ListManagedPortfoliosQueryError {
    #[error("limit must be between 1 and 200")]
    InvalidLimit,
}
