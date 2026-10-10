use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    recipes::{RecipeDefinition, RecipeValidationReport},
    serialization::deserialize_optional_from_string,
    wallet_portfolio::WalletAddress,
};

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeStrategyRequest {
    pub prompt: String,
    #[schema(value_type = Option<String>, format = "evm-address")]
    pub wallet: Option<WalletAddress>,
    /// The intent returned by an earlier composition, sent back to refine it.
    pub previous: Option<StrategyIntent>,
}

/// A strategy in human terms: symbols and decimal amounts, ordered as a chain.
///
/// Addresses, raw units and graph edges are deliberately absent. They are
/// derived by Stockfront from live data so a language model cannot invent them.
#[derive(utoipa::ToSchema, Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct StrategyIntent {
    pub steps: Vec<StrategyStep>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StrategyStep {
    WalletAsset {
        symbol: String,
        #[serde(
            default,
            deserialize_with = "deserialize_optional_from_string",
            skip_serializing_if = "Option::is_none"
        )]
        #[schema(value_type = Option<String>)]
        amount: Option<Decimal>,
    },
    SuppliedPosition {
        symbol: String,
    },
    BorrowedPosition {
        symbol: String,
    },
    Trade {
        #[serde(rename = "buySymbol")]
        buy_symbol: String,
    },
    Supply,
    Withdraw,
    Borrow {
        symbol: String,
        #[serde(
            default,
            deserialize_with = "deserialize_optional_from_string",
            skip_serializing_if = "Option::is_none"
        )]
        #[schema(value_type = Option<String>)]
        amount: Option<Decimal>,
    },
    Repay,
    EarnDeposit {
        symbol: String,
    },
}

/// A language model's proposal before Stockfront resolves and validates it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyDraft {
    pub summary: String,
    pub intent: StrategyIntent,
    pub assumptions: Vec<String>,
}

/// Everything the model may use to draft a strategy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposeContext {
    pub prompt: String,
    /// Whether one stablecoin may be traded for another, which the deployment
    /// decides; the grammar the model is given has to match it.
    pub settlement_pairs: bool,
    pub assets: Vec<ComposerAsset>,
    pub positions: Vec<ComposerPosition>,
    pub previous: Option<StrategyIntent>,
    /// The model's rejected draft and the reasons, when repairing.
    pub rejected: Option<RejectedDraft>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedDraft {
    pub intent: StrategyIntent,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerAsset {
    pub symbol: String,
    pub name: String,
    pub kind: ComposerAssetKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComposerAssetKind {
    Stock,
    Stablecoin { borrowable: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerPosition {
    pub kind: ComposerPositionKind,
    pub symbol: String,
    pub amount: Decimal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComposerPositionKind {
    Supplied,
    Borrowed,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposedStrategy {
    pub summary: String,
    pub intent: StrategyIntent,
    /// Present only when the strategy resolved and passed recipe validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe: Option<RecipeDefinition>,
    pub assumptions: Vec<String>,
    pub missing_inputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<RecipeValidationReport>,
}
