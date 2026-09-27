use alloy_primitives::Address;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorrowingVenue {
    Venus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingToken {
    #[serde(serialize_with = "serialize_as_string")]
    pub contract: Address,
    pub symbol: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingMarket {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub collateral: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
    pub collateral_market: Address,
    pub debt: BorrowingToken,
    #[serde(serialize_with = "serialize_as_string")]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingAsset {
    pub token: BorrowingToken,
    pub amount: Decimal,
    pub value_usd: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowingPosition {
    #[serde(serialize_with = "serialize_as_string")]
    pub pool: Address,
    pub supplied: Vec<BorrowingAsset>,
    pub borrowed: Vec<BorrowingAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Eq, PartialEq, Serialize)]
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
