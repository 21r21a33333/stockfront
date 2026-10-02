//! Live recipe preview and preparation.
//!
//! The compiler stops at a logical plan. This service is the boundary that may
//! call external systems to obtain quotes and prepared wallet actions.
//!
//! Preview and prepare are separate on purpose. A route shown during preview
//! can expire. Prepare therefore fetches current routes again and accepts the
//! selection only when both its route ID and venue still match.

use std::{collections::HashMap, sync::Arc};

use alloy_primitives::Address;
use thiserror::Error;
use uuid::Uuid;

use super::{
    AssetRef, LogicalPlan, PrepareRecipeRequest, PreparedRecipe, PreparedRecipeStep, RecipeAction,
    RecipeCatalog, RecipeDefinition, RecipeNodePreview, RecipeNodeSelection, RecipeNodeState,
    RecipePreview, RecipePreviewRequest, RecipePrimitiveKind, RecipeValidationReport,
    SCHEMA_VERSION, compile,
};
use crate::{
    bstock_market::{BstockMarketError, BstockMarketService, ListBstocksQuery, MAX_LIST_LIMIT},
    bstock_trade::{
        ApprovalRequest, BstockTradeService, PrepareTradeRequest, TradeError, TradeQuoteRequest,
    },
};

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("the recipe is invalid")]
    InvalidRecipe(RecipeValidationReport),
    #[error("the recipe selection is missing, duplicated, or belongs to another node")]
    InvalidSelection,
    #[error("the selected route is no longer available")]
    SelectionExpired,
    #[error("this recipe step depends on the result of an earlier transaction")]
    StepNotReady,
    #[error(transparent)]
    Market(#[from] BstockMarketError),
    #[error(transparent)]
    Trade(#[from] TradeError),
}

pub struct RecipeService {
    market: Arc<BstockMarketService>,
    trade: Arc<BstockTradeService>,
    settlement_tokens: Vec<Address>,
}

impl RecipeService {
    #[must_use]
    pub fn new(
        market: Arc<BstockMarketService>,
        trade: Arc<BstockTradeService>,
        settlement_tokens: Vec<Address>,
    ) -> Self {
        Self {
            market,
            trade,
            settlement_tokens,
        }
    }

    /// Returns the language schema and asset choices needed to render nodes.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::Market`] when the bStock catalog is unavailable.
    pub async fn catalog(&self) -> Result<RecipeCatalog, RecipeError> {
        let bstocks = self
            .market
            .list(&ListBstocksQuery {
                search: None,
                limit: MAX_LIST_LIMIT,
            })
            .await?;
        Ok(RecipeCatalog {
            schema_version: SCHEMA_VERSION,
            primitives: RecipePrimitiveKind::ALL
                .into_iter()
                .map(RecipePrimitiveKind::descriptor)
                .collect(),
            settlement_tokens: self.settlement_tokens.clone(),
            bstocks,
        })
    }

    /// Runs the pure compiler without contacting a provider.
    #[must_use]
    pub fn validate(&self, recipe: &RecipeDefinition) -> RecipeValidationReport {
        compile(recipe).report
    }

    /// Compiles a recipe and resolves the steps that can be quoted now.
    ///
    /// A trade fed directly from a wallet asset is `READY`. A trade fed by the
    /// output of an earlier trade is `WAITING_FOR_UPSTREAM`, because its exact
    /// sell amount does not exist until that earlier transaction completes.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError`] when the recipe is invalid or a required quote
    /// provider is unavailable.
    pub async fn preview(
        &self,
        request: &RecipePreviewRequest,
    ) -> Result<RecipePreview, RecipeError> {
        let plan = compile_plan(&request.recipe)?;
        let mut nodes = Vec::with_capacity(plan.steps.len());
        for step in plan.steps {
            match step.action {
                RecipeAction::Trade { sell, buy_token } => match sell {
                    AssetRef::Wallet { token, amount } => {
                        let routes = self
                            .trade
                            .quotes(&TradeQuoteRequest {
                                wallet: request.context.wallet,
                                sell_token: token,
                                buy_token,
                                sell_amount: amount,
                            })
                            .await?;
                        nodes.push(RecipeNodePreview::Trade {
                            node_id: step.node_id,
                            state: RecipeNodeState::Ready,
                            routes,
                            waiting_for: Vec::new(),
                        });
                    }
                    AssetRef::NodeOutput { .. } => {
                        nodes.push(RecipeNodePreview::Trade {
                            node_id: step.node_id,
                            state: RecipeNodeState::WaitingForUpstream,
                            routes: Vec::new(),
                            waiting_for: step.dependencies,
                        });
                    }
                },
            }
        }
        Ok(RecipePreview { nodes })
    }

    /// Resolves selected preview routes into wallet actions.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError`] when the recipe or selections are invalid, a
    /// route expired, or a provider rejects preparation.
    pub async fn prepare(
        &self,
        request: &PrepareRecipeRequest,
    ) -> Result<PreparedRecipe, RecipeError> {
        let plan = compile_plan(&request.recipe)?;
        // A later step needs the real output of an earlier transaction. Do not
        // prepare a partial batch that cannot be executed as returned.
        if plan.steps.iter().any(|step| {
            matches!(
                step.action,
                RecipeAction::Trade {
                    sell: AssetRef::NodeOutput { .. },
                    ..
                }
            )
        }) {
            return Err(RecipeError::StepNotReady);
        }
        let selections = selections_by_node(&request.selections)?;
        if selections.len() != plan.steps.len() {
            return Err(RecipeError::InvalidSelection);
        }

        let mut prepared = Vec::with_capacity(plan.steps.len() * 2);
        for step in plan.steps {
            let selection = selections
                .get(&step.node_id)
                .copied()
                .ok_or(RecipeError::InvalidSelection)?;
            match (&step.action, selection) {
                (
                    RecipeAction::Trade {
                        sell: AssetRef::Wallet { token, amount },
                        buy_token,
                    },
                    RecipeNodeSelection::Trade {
                        route_quote_id,
                        venue,
                        ..
                    },
                ) => {
                    let trade = TradeQuoteRequest {
                        wallet: request.context.wallet,
                        sell_token: *token,
                        buy_token: *buy_token,
                        sell_amount: *amount,
                    };
                    let still_available =
                        self.trade.quotes(&trade).await?.into_iter().any(|quote| {
                            quote.route_quote_id == *route_quote_id && quote.venue == *venue
                        });
                    if !still_available {
                        return Err(RecipeError::SelectionExpired);
                    }
                    let approval = self
                        .trade
                        .prepare_approval(&ApprovalRequest {
                            token: *token,
                            amount: *amount,
                            venue: venue.clone(),
                        })
                        .await?;
                    let trade = self
                        .trade
                        .prepare_trade(&PrepareTradeRequest {
                            trade,
                            route_quote_id: route_quote_id.clone(),
                        })
                        .await?;
                    prepared.push(PreparedRecipeStep::Approval {
                        node_id: step.node_id,
                        action: approval,
                    });
                    prepared.push(PreparedRecipeStep::Trade {
                        node_id: step.node_id,
                        action: trade,
                    });
                }
                (
                    RecipeAction::Trade {
                        sell: AssetRef::NodeOutput { .. },
                        ..
                    },
                    _,
                ) => {
                    return Err(RecipeError::StepNotReady);
                }
            }
        }
        Ok(PreparedRecipe { steps: prepared })
    }
}

fn compile_plan(recipe: &RecipeDefinition) -> Result<LogicalPlan, RecipeError> {
    let compiled = compile(recipe);
    match compiled.plan {
        Some(plan) => Ok(plan),
        None => Err(RecipeError::InvalidRecipe(compiled.report)),
    }
}

fn selections_by_node(
    selections: &[RecipeNodeSelection],
) -> Result<HashMap<Uuid, &RecipeNodeSelection>, RecipeError> {
    let mut by_node = HashMap::with_capacity(selections.len());
    for selection in selections {
        if by_node.insert(selection.node_id(), selection).is_some() {
            return Err(RecipeError::InvalidSelection);
        }
    }
    Ok(by_node)
}
