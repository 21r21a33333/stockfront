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
    bstock_market::BstockSummary,
    bstock_trade::{PreparedApproval, PreparedTrade, TokenAmount, TradeQuote},
    serialization::{deserialize_from_string, serialize_all_as_strings, serialize_as_string},
    wallet_portfolio::WalletAddress,
};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePrimitiveKind {
    WalletAsset,
    Trade,
}

impl RecipePrimitiveKind {
    pub(super) const ALL: [Self; 2] = [Self::WalletAsset, Self::Trade];
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeFieldControl {
    Token,
    Amount,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeOptionSource {
    SettlementTokens,
    Bstocks,
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortDirection {
    Input,
    Output,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipePortValueType {
    TokenAmount,
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
    Trade {
        #[serde(
            rename = "buyToken",
            deserialize_with = "deserialize_from_string",
            serialize_with = "serialize_as_string"
        )]
        #[schema(value_type = String, format = "evm-address")]
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
    pub trade_policy: RecipeTradePolicy,
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
