//! Natural-language strategy composition on top of the recipe language.
//!
//! A language model only drafts a [`StrategyIntent`]. Resolution to a recipe and
//! validation stay deterministic, so the model can never introduce an address,
//! raw amount or edge that live data and recipe policy have not produced.

mod model;
mod ports;
mod resolve;
mod service;

pub use model::{
    ComposeContext, ComposeStrategyRequest, ComposedStrategy, ComposerAsset, ComposerAssetKind,
    ComposerPosition, ComposerPositionKind, RejectedDraft, SettlementTokenMetadata, StrategyDraft,
    StrategyIntent, StrategyStep,
};
pub use ports::{StrategyModel, StrategyModelError};
pub use service::{StrategyComposerError, StrategyComposerService};
