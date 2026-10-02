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
//! Port names are strings in saved recipes so future schema versions can add
//! ports without changing the JSON shape. The primitive catalog remains the
//! source of truth for which port names and connections are valid.

use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    bstock_market::BstockSummary,
    bstock_trade::{PreparedApproval, PreparedTrade, TradeQuote},
    serialization::{deserialize_from_string, serialize_all_as_strings, serialize_as_string},
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePrimitiveKind {
    WalletAsset,
    Trade,
}

impl RecipePrimitiveKind {
    pub(super) const ALL: [Self; 2] = [Self::WalletAsset, Self::Trade];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeFieldControl {
    Token,
    Amount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeOptionSource {
    SettlementTokens,
    Bstocks,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortValueType {
    TokenAmount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortFlow {
    /// The value can be connected once because the action spends it.
    Consume,
    /// The value may be read by more than one node without spending it.
    Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeFieldDescriptor {
    pub key: &'static str,
    pub name: &'static str,
    pub control: RecipeFieldControl,
    pub required: bool,
    pub option_sources: Vec<RecipeOptionSource>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePortDescriptor {
    pub key: &'static str,
    pub direction: RecipePortDirection,
    pub value_type: RecipePortValueType,
    pub min_connections: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u8>,
    pub flow: RecipePortFlow,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
            Self::WalletAsset => RecipePrimitiveDescriptor {
                kind: self,
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
                ports: vec![RecipePortDescriptor {
                    key: "asset",
                    direction: RecipePortDirection::Output,
                    value_type: RecipePortValueType::TokenAmount,
                    min_connections: 0,
                    max_connections: Some(1),
                    flow: RecipePortFlow::Consume,
                }],
            },
            Self::Trade => RecipePrimitiveDescriptor {
                kind: self,
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
                    RecipePortDescriptor {
                        key: "sell",
                        direction: RecipePortDirection::Input,
                        value_type: RecipePortValueType::TokenAmount,
                        min_connections: 1,
                        max_connections: Some(1),
                        flow: RecipePortFlow::Consume,
                    },
                    RecipePortDescriptor {
                        key: "buy",
                        direction: RecipePortDirection::Output,
                        value_type: RecipePortValueType::TokenAmount,
                        min_connections: 0,
                        max_connections: Some(1),
                        flow: RecipePortFlow::Consume,
                    },
                ],
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePrimitive {
    WalletAsset {
        #[serde(
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        token: Address,
        #[serde(
            rename = "rawAmount",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        raw_amount: U256,
    },
    Trade {
        #[serde(
            rename = "buyToken",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        buy_token: Address,
    },
}

impl RecipePrimitive {
    pub(super) const fn kind(&self) -> RecipePrimitiveKind {
        match self {
            Self::WalletAsset { .. } => RecipePrimitiveKind::WalletAsset,
            Self::Trade { .. } => RecipePrimitiveKind::Trade,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePortRef {
    pub node_id: Uuid,
    pub port: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeEdge {
    pub id: Uuid,
    pub from: RecipePortRef,
    pub to: RecipePortRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDefinition {
    pub schema_version: u16,
    pub chain_id: u64,
    pub nodes: Vec<RecipeNode>,
    pub edges: Vec<RecipeEdge>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeContext {
    pub wallet: WalletAddress,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeCatalog {
    pub schema_version: u16,
    pub primitives: Vec<RecipePrimitiveDescriptor>,
    #[serde(serialize_with = "serialize_all_as_strings")]
    pub settlement_tokens: Vec<Address>,
    pub bstocks: Vec<BstockSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePreviewRequest {
    pub recipe: RecipeDefinition,
    pub context: RecipeContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodeState {
    Ready,
    WaitingForUpstream,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodePreview {
    Trade {
        node_id: Uuid,
        state: RecipeNodeState,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        routes: Vec<TradeQuote>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        waiting_for: Vec<Uuid>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipePreview {
    pub nodes: Vec<RecipeNodePreview>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeNodeSelection {
    Trade {
        node_id: Uuid,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareRecipeRequest {
    pub recipe: RecipeDefinition,
    pub context: RecipeContext,
    pub selections: Vec<RecipeNodeSelection>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreparedRecipeStep {
    Approval {
        node_id: Uuid,
        action: PreparedApproval,
    },
    Trade {
        node_id: Uuid,
        action: PreparedTrade,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRecipe {
    pub steps: Vec<PreparedRecipeStep>,
}
