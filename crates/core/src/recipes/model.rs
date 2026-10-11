//! Public recipe types used by the editor and HTTP API.
//!
//! A recipe describes *what* should happen. It does not contain the wallet that
//! will run it. This lets one saved recipe be previewed or prepared for more
//! than one wallet.
//!
//! For example, this edge sends the `asset` output of a wallet source into the
//! `sell` input of a trade:
//!
//! ```text
//! Wallet asset [asset] ──> [sell] Trade [buy]
//! ```
//!
//! The saved recipe and runtime context stay separate:
//!
//! ```json
//! {
//!   "recipe": {
//!     "schemaVersion": 1,
//!     "chainId": 56,
//!     "nodes": [
//!       {
//!         "id": "00000000-0000-0000-0000-000000000001",
//!         "kind": "WALLET_ASSET",
//!         "token": "0x55d398326f99059fF775485246999027B3197955",
//!         "rawAmount": "6000000000000000000"
//!       },
//!       {
//!         "id": "00000000-0000-0000-0000-000000000002",
//!         "kind": "TRADE",
//!         "buyToken": "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436"
//!       }
//!     ],
//!     "edges": [
//!       {
//!         "id": "00000000-0000-0000-0000-000000000003",
//!         "from": { "nodeId": "00000000-0000-0000-0000-000000000001", "port": "asset" },
//!         "to": { "nodeId": "00000000-0000-0000-0000-000000000002", "port": "sell" }
//!       }
//!     ]
//!   },
//!   "context": { "wallet": "0x0000000000000000000000000000000000000001" }
//! }
//! ```
//!
//! After an action settles, the client sends its exact output in the runtime
//! context. This unlocks only the next dependent action. A completed action
//! with no output stays recorded, but its dependent action remains blocked:
//!
//! ```json
//! {
//!   "wallet": "0x0000000000000000000000000000000000000001",
//!   "completedNodes": [{
//!     "nodeId": "00000000-0000-0000-0000-000000000002",
//!     "outputs": [{
//!       "port": "buy",
//!       "asset": {
//!         "token": "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
//!         "amount": "17456456017850632"
//!       }
//!     }]
//!   }]
//! }
//! ```
//!
//! Port names are strings in saved recipes so future schema versions can add
//! ports without changing the JSON shape. The primitive catalog remains the
//! source of truth for which port names and connections are valid.

use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    baskets::BasketSummary,
    bstock_borrowing::{
        BorrowCapability, BorrowingAsset, BorrowingVenue, PreparedBorrowingAction,
        RepaymentCapability, SupplyMarket,
    },
    bstock_market::BstockSummary,
    bstock_trade::{PreparedApproval, PreparedTrade, TokenAmount, TradeQuote},
    carry::{CarryActionPlan, CarryOpportunity, CarryPosition, PreparedCarryDeposit},
    serialization::{deserialize_from_string, serialize_all_as_strings, serialize_as_string},
    wallet_portfolio::WalletAddress,
};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePrimitiveKind {
    WalletAsset,
    SuppliedPosition,
    BorrowedPosition,
    Trade,
    Supply,
    Withdraw,
    Borrow,
    Repay,
    EarnDeposit,
    EarnPosition,
    EarnWithdraw,
}

impl RecipePrimitiveKind {
    pub(super) const ALL: [Self; 11] = [
        Self::WalletAsset,
        Self::SuppliedPosition,
        Self::BorrowedPosition,
        Self::Trade,
        Self::Supply,
        Self::Withdraw,
        Self::Borrow,
        Self::Repay,
        Self::EarnDeposit,
        Self::EarnPosition,
        Self::EarnWithdraw,
    ];
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeFieldControl {
    Token,
    Amount,
    Venue,
    Position,
    Investment,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeOptionSource {
    SettlementTokens,
    Bstocks,
    SuppliedPositions,
    BorrowedPositions,
    EarnOpportunities,
    EarnPositions,
    Baskets,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeAssetTransition {
    pub sell: RecipeOptionSource,
    pub buy: RecipeOptionSource,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeTradePolicy {
    pub allowed_transitions: Vec<RecipeAssetTransition>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSupplyRule {
    pub venue: BorrowingVenue,
    pub input: RecipeOptionSource,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSupplyPolicy {
    pub allowed_inputs: Vec<RecipeSupplyRule>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeWithdrawalRule {
    pub venue: BorrowingVenue,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeWithdrawalPolicy {
    pub allowed_inputs: Vec<RecipeWithdrawalRule>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBorrowRule {
    pub venue: BorrowingVenue,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
    pub input: RecipeOptionSource,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBorrowPolicy {
    pub allowed_inputs: Vec<RecipeBorrowRule>,
    #[serde(serialize_with = "serialize_all_as_strings")]
    #[schema(value_type = Vec<String>, format = "evm-address")]
    pub debt_tokens: Vec<Address>,
    pub minimum_health_factor: rust_decimal::Decimal,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRepayRule {
    pub venue: BorrowingVenue,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub pool: Address,
    pub input: RecipeOptionSource,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRepayPolicy {
    pub allowed_inputs: Vec<RecipeRepayRule>,
    #[serde(serialize_with = "serialize_all_as_strings")]
    #[schema(value_type = Vec<String>, format = "evm-address")]
    pub debt_tokens: Vec<Address>,
}

/// Structural policy for earn deposits.
///
/// Reviewed investment IDs and spread qualification stay backend-owned live
/// data. Only the graph shapes the editor may build are published here.
#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeEarnDepositPolicy {
    pub allowed_inputs: Vec<RecipePrimitiveKind>,
    #[serde(serialize_with = "serialize_all_as_strings")]
    #[schema(value_type = Vec<String>, format = "evm-address")]
    pub deposit_tokens: Vec<Address>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortDirection {
    Input,
    Output,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortValueType {
    TokenAmount,
    SuppliedPosition,
    BorrowedPosition,
    EarnPosition,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortFlow {
    /// The value can be connected once because the action spends it.
    Consume,
    /// The value may be read by more than one node without spending it.
    Reference,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeFieldDescriptor {
    pub key: &'static str,
    pub name: &'static str,
    pub control: RecipeFieldControl,
    pub required: bool,
    pub option_sources: Vec<RecipeOptionSource>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePortDescriptor {
    pub key: &'static str,
    pub direction: RecipePortDirection,
    pub value_type: RecipePortValueType,
    pub min_connections: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub max_connections: Option<u8>,
    pub flow: RecipePortFlow,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePrimitiveDescriptor {
    pub kind: RecipePrimitiveKind,
    pub name: &'static str,
    pub description: &'static str,
    pub fields: Vec<RecipeFieldDescriptor>,
    pub ports: Vec<RecipePortDescriptor>,
}

impl RecipePrimitiveKind {
    #[must_use]
    pub fn descriptor(self) -> RecipePrimitiveDescriptor {
        match self {
            Self::WalletAsset => wallet_asset_descriptor(self),
            Self::SuppliedPosition => supplied_position_descriptor(self),
            Self::BorrowedPosition => borrowed_position_descriptor(self),
            Self::Trade => trade_descriptor(self),
            Self::Supply => supply_descriptor(self),
            Self::Withdraw => withdraw_descriptor(self),
            Self::Borrow => borrow_descriptor(self),
            Self::Repay => repay_descriptor(self),
            Self::EarnDeposit => earn_deposit_descriptor(self),
            Self::EarnPosition => earn_position_descriptor(self),
            Self::EarnWithdraw => earn_withdraw_descriptor(self),
        }
    }
}

fn wallet_asset_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Wallet asset",
        description: "Use a token amount held by the running wallet.",
        fields: vec![
            RecipeFieldDescriptor {
                key: "token",
                name: "Asset",
                control: RecipeFieldControl::Token,
                required: true,
                option_sources: vec![
                    RecipeOptionSource::SettlementTokens,
                    RecipeOptionSource::Bstocks,
                ],
            },
            RecipeFieldDescriptor {
                key: "rawAmount",
                name: "Amount",
                control: RecipeFieldControl::Amount,
                required: true,
                option_sources: Vec::new(),
            },
        ],
        ports: vec![output_port("asset", RecipePortValueType::TokenAmount)],
    }
}

fn supplied_position_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Supplied position",
        description: "Use a supported position already supplied by the running wallet.",
        fields: vec![RecipeFieldDescriptor {
            key: "token",
            name: "Position",
            control: RecipeFieldControl::Position,
            required: true,
            option_sources: vec![RecipeOptionSource::SuppliedPositions],
        }],
        ports: vec![output_port(
            "position",
            RecipePortValueType::SuppliedPosition,
        )],
    }
}

fn borrowed_position_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Borrowed position",
        description: "Use a supported debt position held by the running wallet.",
        fields: vec![RecipeFieldDescriptor {
            key: "token",
            name: "Debt position",
            control: RecipeFieldControl::Position,
            required: true,
            option_sources: vec![RecipeOptionSource::BorrowedPositions],
        }],
        ports: vec![output_port("debt", RecipePortValueType::BorrowedPosition)],
    }
}

fn trade_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Trade",
        description: "Exchange the incoming asset for another supported asset.",
        fields: vec![RecipeFieldDescriptor {
            key: "buyToken",
            name: "Buy asset",
            control: RecipeFieldControl::Token,
            required: true,
            option_sources: vec![
                RecipeOptionSource::SettlementTokens,
                RecipeOptionSource::Bstocks,
            ],
        }],
        ports: vec![
            input_port("sell", RecipePortValueType::TokenAmount),
            output_port("buy", RecipePortValueType::TokenAmount),
        ],
    }
}

fn supply_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Supply",
        description: "Supply the incoming bStock to a supported lending market.",
        fields: vec![RecipeFieldDescriptor {
            key: "venue",
            name: "Market",
            control: RecipeFieldControl::Venue,
            required: true,
            option_sources: Vec::new(),
        }],
        ports: vec![
            input_port("asset", RecipePortValueType::TokenAmount),
            output_port("position", RecipePortValueType::SuppliedPosition),
        ],
    }
}

fn earn_deposit_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Earn deposit",
        description: "Deposit the incoming stablecoin into a reviewed earn opportunity.",
        fields: vec![
            RecipeFieldDescriptor {
                key: "collateralToken",
                name: "Benchmark collateral",
                control: RecipeFieldControl::Token,
                required: true,
                option_sources: vec![RecipeOptionSource::Bstocks],
            },
            RecipeFieldDescriptor {
                key: "investmentId",
                name: "Earn opportunity",
                control: RecipeFieldControl::Investment,
                required: true,
                option_sources: vec![RecipeOptionSource::EarnOpportunities],
            },
        ],
        ports: vec![input_port("asset", RecipePortValueType::TokenAmount)],
    }
}

fn withdraw_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Withdraw",
        description: "Withdraw the complete supplied position to the running wallet.",
        fields: Vec::new(),
        ports: vec![input_port(
            "position",
            RecipePortValueType::SuppliedPosition,
        )],
    }
}

fn earn_position_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Earn position",
        description: "An existing earn deposit held by the running wallet.",
        fields: vec![RecipeFieldDescriptor {
            key: "investmentId",
            name: "Earn position",
            control: RecipeFieldControl::Position,
            required: true,
            option_sources: vec![RecipeOptionSource::EarnPositions],
        }],
        ports: vec![output_port("position", RecipePortValueType::EarnPosition)],
    }
}

fn earn_withdraw_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Withdraw from earn",
        description: "Withdraw the complete earn position to the running wallet.",
        fields: Vec::new(),
        ports: vec![input_port("position", RecipePortValueType::EarnPosition)],
    }
}

fn borrow_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Borrow stablecoin",
        description: "Borrow a supported stablecoin against an existing supplied position.",
        fields: vec![
            RecipeFieldDescriptor {
                key: "debtToken",
                name: "Borrow asset",
                control: RecipeFieldControl::Token,
                required: true,
                option_sources: vec![RecipeOptionSource::SettlementTokens],
            },
            RecipeFieldDescriptor {
                key: "rawAmount",
                name: "Amount",
                control: RecipeFieldControl::Amount,
                required: true,
                option_sources: Vec::new(),
            },
        ],
        ports: vec![
            input_port("collateral", RecipePortValueType::SuppliedPosition),
            output_port("borrowed", RecipePortValueType::TokenAmount),
        ],
    }
}

fn repay_descriptor(kind: RecipePrimitiveKind) -> RecipePrimitiveDescriptor {
    RecipePrimitiveDescriptor {
        kind,
        name: "Repay debt",
        description: "Fully repay the incoming Venus debt position.",
        fields: Vec::new(),
        ports: vec![input_port("debt", RecipePortValueType::BorrowedPosition)],
    }
}

const fn input_port(key: &'static str, value_type: RecipePortValueType) -> RecipePortDescriptor {
    RecipePortDescriptor {
        key,
        direction: RecipePortDirection::Input,
        value_type,
        min_connections: 1,
        max_connections: Some(1),
        flow: RecipePortFlow::Consume,
    }
}

const fn output_port(key: &'static str, value_type: RecipePortValueType) -> RecipePortDescriptor {
    RecipePortDescriptor {
        key,
        direction: RecipePortDirection::Output,
        value_type,
        min_connections: 0,
        max_connections: Some(1),
        flow: RecipePortFlow::Consume,
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePrimitive {
    WalletAsset {
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        token: Address,
        #[serde(
            rename = "rawAmount",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String)]
        raw_amount: U256,
    },
    SuppliedPosition {
        venue: BorrowingVenue,
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        pool: Address,
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        token: Address,
    },
    BorrowedPosition {
        venue: BorrowingVenue,
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        pool: Address,
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        token: Address,
    },
    Trade {
        #[serde(
            rename = "buyToken",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        buy_token: Address,
    },
    Supply {
        venue: BorrowingVenue,
    },
    Withdraw,
    Borrow {
        #[serde(
            rename = "debtToken",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        debt_token: Address,
        #[serde(
            rename = "rawAmount",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String)]
        raw_amount: U256,
    },
    Repay,
    EarnDeposit {
        #[serde(
            rename = "collateralToken",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
        collateral_token: Address,
        #[serde(rename = "investmentId")]
        investment_id: String,
    },
    EarnPosition {
        #[serde(rename = "investmentId")]
        investment_id: String,
    },
    EarnWithdraw,
}

impl RecipePrimitive {
    pub(super) const fn kind(&self) -> RecipePrimitiveKind {
        match self {
            Self::WalletAsset { .. } => RecipePrimitiveKind::WalletAsset,
            Self::SuppliedPosition { .. } => RecipePrimitiveKind::SuppliedPosition,
            Self::BorrowedPosition { .. } => RecipePrimitiveKind::BorrowedPosition,
            Self::Trade { .. } => RecipePrimitiveKind::Trade,
            Self::Supply { .. } => RecipePrimitiveKind::Supply,
            Self::Withdraw => RecipePrimitiveKind::Withdraw,
            Self::Borrow { .. } => RecipePrimitiveKind::Borrow,
            Self::Repay => RecipePrimitiveKind::Repay,
            Self::EarnDeposit { .. } => RecipePrimitiveKind::EarnDeposit,
            Self::EarnPosition { .. } => RecipePrimitiveKind::EarnPosition,
            Self::EarnWithdraw => RecipePrimitiveKind::EarnWithdraw,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeNode {
    pub id: Uuid,
    #[serde(flatten)]
    pub primitive: RecipePrimitive,
}

impl RecipeNode {
    pub(super) fn descriptor(&self) -> RecipePrimitiveDescriptor {
        self.primitive.kind().descriptor()
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePortRef {
    pub node_id: Uuid,
    pub port: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeEdge {
    pub id: Uuid,
    pub from: RecipePortRef,
    pub to: RecipePortRef,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDefinition {
    pub schema_version: u16,
    pub chain_id: u64,
    pub nodes: Vec<RecipeNode>,
    pub edges: Vec<RecipeEdge>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeContext {
    #[schema(value_type = String, format = "evm-address")]
    pub wallet: WalletAddress,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completed_nodes: Vec<RecipeCompletedNode>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeCompletedNode {
    pub node_id: Uuid,
    pub outputs: Vec<RecipeNodeOutput>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeNodeOutput {
    pub port: String,
    pub asset: TokenAmount,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeCatalog {
    pub schema_version: u16,
    pub primitives: Vec<RecipePrimitiveDescriptor>,
    #[serde(serialize_with = "serialize_all_as_strings")]
    #[schema(value_type = Vec<String>, format = "evm-address")]
    pub settlement_tokens: Vec<Address>,
    pub bstocks: Vec<BstockSummary>,
    pub baskets: Vec<BasketSummary>,
    pub trade_policy: RecipeTradePolicy,
    pub supply_policy: RecipeSupplyPolicy,
    pub withdrawal_policy: RecipeWithdrawalPolicy,
    pub borrow_policy: RecipeBorrowPolicy,
    pub repay_policy: RecipeRepayPolicy,
    pub earn_deposit_policy: RecipeEarnDepositPolicy,
}

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePreviewRequest {
    pub recipe: RecipeDefinition,
    pub context: RecipeContext,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodeState {
    Ready,
    WaitingForUpstream,
    Complete,
    OutputUnavailable,
    Unavailable,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeWithdrawalUnavailableReason {
    PositionNotFound,
    UnsupportedPool,
    OutstandingDebt,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodePreview {
    Trade {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        routes: Vec<TradeQuote>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    Supply {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Option::is_none")]
        market: Option<SupplyMarket>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    Withdraw {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<BorrowingAsset>,
        #[serde(rename = "unavailableReason", skip_serializing_if = "Option::is_none")]
        unavailable_reason: Option<RecipeWithdrawalUnavailableReason>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    EarnWithdraw {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<CarryPosition>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    Borrow {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Option::is_none")]
        capability: Option<BorrowCapability>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    Repay {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Option::is_none")]
        capability: Option<RepaymentCapability>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
    EarnDeposit {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        state: RecipeNodeState,
        /// The selected route, carrying its own qualification and reasons when
        /// the investment is known but not currently depositable.
        #[serde(skip_serializing_if = "Option::is_none")]
        opportunity: Option<CarryOpportunity>,
        #[serde(rename = "waitingFor", skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePreview {
    pub nodes: Vec<RecipeNodePreview>,
}

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodeSelection {
    Trade {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        #[serde(rename = "routeQuoteId")]
        route_quote_id: String,
        venue: String,
    },
}

impl RecipeNodeSelection {
    pub(super) const fn node_id(&self) -> Uuid {
        match self {
            Self::Trade { node_id, .. } => *node_id,
        }
    }
}

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareRecipeRequest {
    pub recipe: RecipeDefinition,
    pub context: RecipeContext,
    pub selections: Vec<RecipeNodeSelection>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreparedRecipeStep {
    Approval {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedApproval,
    },
    Trade {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedTrade,
    },
    BasketTrade {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedTrade,
    },
    Supply {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedBorrowingAction,
    },
    Withdraw {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedBorrowingAction,
    },
    EarnWithdraw {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        #[schema(inline)]
        action: CarryActionPlan,
    },
    EnableCollateral {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedBorrowingAction,
    },
    Borrow {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedBorrowingAction,
        output: TokenAmount,
    },
    Repay {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: PreparedBorrowingAction,
    },
    EarnDeposit {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        action: Box<PreparedCarryDeposit>,
    },
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRecipe {
    pub steps: Vec<PreparedRecipeStep>,
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, Bytes};
    use serde_json::{from_value, json, to_value};

    use super::{
        PreparedRecipe, PreparedRecipeStep, RecipeContext, RecipeNodePreview, RecipeNodeSelection,
        RecipeNodeState,
    };
    use crate::bstock_trade::PreparedApproval;

    #[test]
    fn serializes_preview_node_fields_as_camel_case() {
        let node_id = uuid::Uuid::from_u128(1);
        let result = to_value(RecipeNodePreview::Trade {
            node_id,
            state: RecipeNodeState::Ready,
            routes: Vec::new(),
            waiting_for: Vec::new(),
        });

        let Ok(value) = result else {
            panic!("recipe preview did not serialize");
        };
        assert_eq!(
            value,
            json!({
                "kind": "TRADE",
                "nodeId": node_id,
                "state": "READY"
            })
        );
    }

    #[test]
    fn deserializes_camel_case_recipe_selection_fields() {
        let node_id = uuid::Uuid::from_u128(2);
        let result = from_value::<RecipeNodeSelection>(json!({
            "kind": "TRADE",
            "nodeId": node_id,
            "routeQuoteId": "quote-1",
            "venue": "LiquidMesh"
        }));

        let Ok(RecipeNodeSelection::Trade {
            node_id: actual_node_id,
            route_quote_id,
            venue,
        }) = result
        else {
            panic!("camel-case recipe selection did not deserialize");
        };
        assert_eq!(actual_node_id, node_id);
        assert_eq!(route_quote_id, "quote-1");
        assert_eq!(venue, "LiquidMesh");
    }

    #[test]
    fn serializes_prepared_step_node_id_as_camel_case() {
        let node_id = uuid::Uuid::from_u128(3);
        let result = to_value(PreparedRecipe {
            steps: vec![PreparedRecipeStep::Approval {
                node_id,
                action: PreparedApproval {
                    to: Address::ZERO,
                    spender: Address::ZERO,
                    data: Bytes::new(),
                    gas_limit: None,
                    gas_price: None,
                },
            }],
        });

        let Ok(value) = result else {
            panic!("prepared recipe did not serialize");
        };
        assert_eq!(value.pointer("/steps/0/nodeId"), Some(&json!(node_id)));
        assert!(value.pointer("/steps/0/node_id").is_none());
    }

    #[test]
    fn defaults_missing_completed_nodes_for_existing_clients() {
        let result = from_value::<RecipeContext>(json!({
            "wallet": "0x0000000000000000000000000000000000000001"
        }));

        let Ok(context) = result else {
            panic!("legacy recipe context did not deserialize");
        };
        assert!(context.completed_nodes.is_empty());
    }
}
