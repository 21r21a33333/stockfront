use alloy_primitives::{Address, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    prepared_action::{ActionWarning, NetworkFee, PreparedTransaction},
    serialization::{deserialize_from_string, serialize_as_string},
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BorrowingMarketQuery {
    pub collateral_token: Option<Address>,
    pub debt_token: Option<Address>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingVenue {
    Venus,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingToken {
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub contract: Address,
    pub symbol: String,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingMarketUnavailableReason {
    MarketUnlisted,
    SupplyPaused,
    SupplyCapReached,
    BorrowPaused,
    BorrowCapReached,
    InsufficientBorrowLiquidity,
    InvalidOraclePrice,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupplyMarketUnavailableReason {
    MarketUnlisted,
    SupplyPaused,
    SupplyCapReached,
}

/// Current Venus capability for supplying one supported bStock.
///
/// `remaining_supply_capacity` is `None` when Venus reports an unlimited cap.
/// This model intentionally excludes collateral and borrowing fields: supplying
/// an asset does not require it to be collateral-eligible or borrowable.
#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplyMarket {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub token: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub market: Address,
    pub token_decimals: u32,
    pub supply_apy_percent: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_supply_capacity: Option<Decimal>,
    pub supply_available: bool,
    pub unavailable_reasons: Vec<SupplyMarketUnavailableReason>,
    pub observed_at_ms: i64,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingMarket {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub collateral: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub collateral_market: Address,
    pub debt: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub debt_market: Address,
    pub collateral_factor: Decimal,
    pub liquidation_threshold: Decimal,
    pub remaining_supply_capacity: Decimal,
    pub available_borrow_liquidity: Decimal,
    pub borrow_apy_percent: Decimal,
    pub supply_available: bool,
    pub borrow_available: bool,
    pub oracle_valid: bool,
    pub unavailable_reasons: Vec<BorrowingMarketUnavailableReason>,
    pub observed_at_ms: i64,
}

/// Live safety and liquidity information for one exact Venus borrow.
///
/// The selected collateral is only the position the recipe starts from. Venus
/// still evaluates every enabled collateral and every debt in the account.
#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowCapability {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
    pub collateral: BorrowingToken,
    pub debt: BorrowingToken,
    pub requested_amount: Decimal,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String)]
    pub requested_raw_amount: U256,
    pub debt_token_decimals: u32,
    pub conservative_max_amount: Decimal,
    pub available_borrow_liquidity: Decimal,
    pub borrow_apy_percent: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor_before: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor_after: Option<Decimal>,
    pub collateral_enabled: bool,
    pub stage: BorrowStage,
    pub unavailable_reasons: Vec<BorrowCapabilityUnavailableReason>,
    pub observed_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BorrowCapabilityRequest {
    pub wallet: WalletAddress,
    pub pool: Address,
    pub collateral_token: Address,
    pub debt_token: Address,
    pub raw_amount: U256,
    pub minimum_health_factor: Decimal,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BorrowStage {
    EnableCollateral,
    Borrow,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BorrowCapabilityUnavailableReason {
    PositionNotFound,
    UnsupportedPool,
    MarketUnlisted,
    CollateralUnavailable,
    BorrowPaused,
    BorrowCapReached,
    InsufficientBorrowLiquidity,
    InvalidOraclePrice,
    HealthFactorUnavailable,
    HealthFactorBelowMinimum,
}

/// Current ability to repay one complete Venus debt position.
///
/// Amounts come from the same live observation. `approval_amount` includes a
/// small interest buffer so a full repayment does not leave debt dust while the
/// wallet request is pending.
#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepaymentCapability {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
    pub debt: BorrowingToken,
    pub debt_amount: Decimal,
    pub wallet_balance: Decimal,
    pub approval_amount: Decimal,
    pub approval_required: bool,
    pub unavailable_reasons: Vec<RepaymentCapabilityUnavailableReason>,
    pub observed_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepaymentCapabilityRequest {
    pub wallet: WalletAddress,
    pub pool: Address,
    pub debt_token: Address,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepaymentCapabilityUnavailableReason {
    PositionNotFound,
    UnsupportedPool,
    UnsupportedDebtToken,
    NoDebt,
    InsufficientWalletBalance,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingAsset {
    pub token: BorrowingToken,
    pub amount: Decimal,
    pub value_usd: Decimal,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingPosition {
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
    pub supplied: Vec<BorrowingAsset>,
    pub borrowed: Vec<BorrowingAsset>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingAccount {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub positions: Vec<BorrowingPosition>,
    pub total_supplied_usd: Decimal,
    pub total_borrowed_usd: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor: Option<Decimal>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareSupplyRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCollateralRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_token: Address,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareBorrowRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub debt_token: Address,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "mode", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionAmount {
    Exact {
        #[serde(deserialize_with = "deserialize_from_string")]
        value: Decimal,
    },
    Full,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareRepaymentRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub debt_token: Address,
    pub amount: PositionAmount,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareWithdrawalRequest {
    pub wallet: WalletAddress,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_token: Address,
    pub amount: PositionAmount,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BorrowingTransactionKind {
    Approve,
    Deposit,
    EnableCollateral,
    Borrow,
    Repay,
    Withdraw,
}

pub type PreparedBorrowingTransaction = PreparedTransaction<BorrowingTransactionKind>;
pub type BorrowingWarning = ActionWarning;
pub type BorrowingNetworkFee = NetworkFee;

#[derive(utoipa::ToSchema, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingPreview {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_network_fee: Option<BorrowingNetworkFee>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor_before: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_factor_after: Option<Decimal>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<BorrowingWarning>,
}

#[derive(utoipa::ToSchema, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedBorrowingAction {
    pub transactions: Vec<PreparedBorrowingTransaction>,
    pub preview: BorrowingPreview,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingTransactionState {
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingTransactionStatus {
    pub state: BorrowingTransactionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_at_ms: Option<i64>,
}
