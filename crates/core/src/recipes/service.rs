//! Live recipe preview and preparation.
//!
//! The compiler stops at a logical plan. This service is the boundary that may
//! call external systems to obtain quotes and prepared wallet actions.
//!
//! Preview and prepare are separate on purpose. Prepare passes the selected
//! provider quote ID back to the provider, which decides whether it is still
//! valid while building the wallet action.

use std::{collections::HashMap, sync::Arc};

use alloy_primitives::Address;
use thiserror::Error;
use uuid::Uuid;

use super::{
    AssetRef, LogicalPlan, PrepareRecipeRequest, PreparedRecipe, PreparedRecipeStep, RecipeAction,
    RecipeAssetTransition, RecipeCatalog, RecipeCompletedNode, RecipeDefinition, RecipeDiagnostic,
    RecipeDiagnosticCode, RecipeDiagnosticLocation, RecipeNodePreview, RecipeNodeSelection,
    RecipeNodeState, RecipeOptionSource, RecipePreview, RecipePreviewRequest, RecipePrimitiveKind,
    RecipeTradePolicy, RecipeValidationReport, SCHEMA_VERSION, compile,
};
use crate::{
    bstock_market::{BstockMarketError, BstockMarketService, ListBstocksQuery, MAX_LIST_LIMIT},
    bstock_trade::{
        ApprovalRequest, BstockTradeService, PrepareTradeRequest, TokenAmount, TradeError,
        TradeQuoteRequest,
    },
};

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("the recipe is invalid")]
    InvalidRecipe(RecipeValidationReport),
    #[error("the recipe runtime context does not match the compiled plan")]
    InvalidContext,
    #[error("the recipe selection is missing, duplicated, or belongs to another node")]
    InvalidSelection,
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
            trade_policy: RecipeTradePolicy {
                allowed_transitions: vec![
                    RecipeAssetTransition {
                        sell: RecipeOptionSource::SettlementTokens,
                        buy: RecipeOptionSource::Bstocks,
                    },
                    RecipeAssetTransition {
                        sell: RecipeOptionSource::Bstocks,
                        buy: RecipeOptionSource::SettlementTokens,
                    },
                ],
            },
        })
    }

    /// Runs the pure compiler without contacting a provider.
    #[must_use]
    pub fn validate(&self, recipe: &RecipeDefinition) -> RecipeValidationReport {
        let compiled = compile(recipe);
        validate_trade_pairs(compiled.report, compiled.plan.as_ref(), &self.trade)
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
        let plan = compile_plan(&request.recipe, &self.trade)?;
        let completed = validate_completed_context(&plan, &request.context.completed_nodes)?;
        let mut nodes = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            if completed.contains_key(&step.node_id) {
                nodes.push(RecipeNodePreview::Trade {
                    node_id: step.node_id,
                    state: RecipeNodeState::Complete,
                    routes: Vec::new(),
                    waiting_for: Vec::new(),
                });
                continue;
            }

            match &step.action {
                RecipeAction::Trade { sell, buy_token } => match resolve_asset(sell, &completed) {
                    ResolvedAsset::Ready(asset) => {
                        let routes = self
                            .trade
                            .quotes(&TradeQuoteRequest {
                                wallet: request.context.wallet,
                                sell_token: asset.token,
                                buy_token: *buy_token,
                                sell_amount: asset.amount,
                            })
                            .await?;
                        nodes.push(RecipeNodePreview::Trade {
                            node_id: step.node_id,
                            state: RecipeNodeState::Ready,
                            routes,
                            waiting_for: Vec::new(),
                        });
                    }
                    ResolvedAsset::Waiting => {
                        nodes.push(RecipeNodePreview::Trade {
                            node_id: step.node_id,
                            state: RecipeNodeState::WaitingForUpstream,
                            routes: Vec::new(),
                            waiting_for: step.dependencies.clone(),
                        });
                    }
                    ResolvedAsset::Unavailable => {
                        nodes.push(RecipeNodePreview::Trade {
                            node_id: step.node_id,
                            state: RecipeNodeState::OutputUnavailable,
                            routes: Vec::new(),
                            waiting_for: step.dependencies.clone(),
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
    /// Returns [`RecipeError`] when the recipe or selections are invalid, or a
    /// provider rejects preparation or reports that its quote expired.
    pub async fn prepare(
        &self,
        request: &PrepareRecipeRequest,
    ) -> Result<PreparedRecipe, RecipeError> {
        let plan = compile_plan(&request.recipe, &self.trade)?;
        let completed = validate_completed_context(&plan, &request.context.completed_nodes)?;
        let ready = plan
            .steps
            .iter()
            .filter_map(|step| {
                if completed.contains_key(&step.node_id) {
                    return None;
                }
                match &step.action {
                    RecipeAction::Trade { sell, .. } => match resolve_asset(sell, &completed) {
                        ResolvedAsset::Ready(asset) => Some((step, asset)),
                        ResolvedAsset::Waiting | ResolvedAsset::Unavailable => None,
                    },
                }
            })
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(RecipeError::StepNotReady);
        }
        let selections = selections_by_node(&request.selections)?;
        if selections.len() != ready.len()
            || ready
                .iter()
                .any(|(step, _)| !selections.contains_key(&step.node_id))
        {
            return Err(RecipeError::InvalidSelection);
        }

        let mut prepared = Vec::with_capacity(ready.len() * 2);
        for (step, sell) in ready {
            let selection = selections
                .get(&step.node_id)
                .copied()
                .ok_or(RecipeError::InvalidSelection)?;
            match (&step.action, selection) {
                (
                    RecipeAction::Trade { buy_token, .. },
                    RecipeNodeSelection::Trade {
                        route_quote_id,
                        venue,
                        ..
                    },
                ) => {
                    let trade = TradeQuoteRequest {
                        wallet: request.context.wallet,
                        sell_token: sell.token,
                        buy_token: *buy_token,
                        sell_amount: sell.amount,
                    };
                    let approval = self
                        .trade
                        .prepare_approval(&ApprovalRequest {
                            token: sell.token,
                            amount: sell.amount,
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
            }
        }
        Ok(PreparedRecipe { steps: prepared })
    }
}

#[derive(Clone, Copy)]
enum ResolvedAsset {
    Ready(TokenAmount),
    Waiting,
    Unavailable,
}

fn resolve_asset(
    asset: &AssetRef,
    completed: &HashMap<Uuid, &RecipeCompletedNode>,
) -> ResolvedAsset {
    match asset {
        AssetRef::Wallet { token, amount } => ResolvedAsset::Ready(TokenAmount {
            token: *token,
            amount: *amount,
        }),
        AssetRef::NodeOutput { node_id, port, .. } => {
            let Some(node) = completed.get(node_id) else {
                return ResolvedAsset::Waiting;
            };
            node.outputs
                .iter()
                .find(|output| output.port == *port)
                .map_or(ResolvedAsset::Unavailable, |output| {
                    ResolvedAsset::Ready(output.asset)
                })
        }
    }
}

fn validate_completed_context<'a>(
    plan: &LogicalPlan,
    completed_nodes: &'a [RecipeCompletedNode],
) -> Result<HashMap<Uuid, &'a RecipeCompletedNode>, RecipeError> {
    let steps = plan
        .steps
        .iter()
        .map(|step| (step.node_id, step))
        .collect::<HashMap<_, _>>();
    let mut completed = HashMap::with_capacity(completed_nodes.len());

    for node in completed_nodes {
        let Some(step) = steps.get(&node.node_id) else {
            return Err(RecipeError::InvalidContext);
        };
        if completed.insert(node.node_id, node).is_some() {
            return Err(RecipeError::InvalidContext);
        }

        let RecipeAction::Trade { buy_token, .. } = &step.action;
        let mut output_seen = false;
        for output in &node.outputs {
            if output_seen
                || output.port != "buy"
                || output.asset.token != *buy_token
                || output.asset.amount.is_zero()
            {
                return Err(RecipeError::InvalidContext);
            }
            output_seen = true;
        }
    }

    for node in completed_nodes {
        let step = steps
            .get(&node.node_id)
            .ok_or(RecipeError::InvalidContext)?;
        if step
            .dependencies
            .iter()
            .any(|dependency| !completed.contains_key(dependency))
        {
            return Err(RecipeError::InvalidContext);
        }
        if let RecipeAction::Trade {
            sell: AssetRef::NodeOutput { node_id, port, .. },
            ..
        } = &step.action
        {
            let dependency = completed.get(node_id).ok_or(RecipeError::InvalidContext)?;
            if !dependency.outputs.iter().any(|output| output.port == *port) {
                return Err(RecipeError::InvalidContext);
            }
        }
    }

    Ok(completed)
}

fn compile_plan(
    recipe: &RecipeDefinition,
    trade: &BstockTradeService,
) -> Result<LogicalPlan, RecipeError> {
    let compiled = compile(recipe);
    match compiled.plan {
        Some(plan) => {
            let report = validate_trade_pairs(compiled.report, Some(&plan), trade);
            if report.valid {
                Ok(plan)
            } else {
                Err(RecipeError::InvalidRecipe(report))
            }
        }
        None => Err(RecipeError::InvalidRecipe(compiled.report)),
    }
}

fn validate_trade_pairs(
    mut report: RecipeValidationReport,
    plan: Option<&LogicalPlan>,
    trade: &BstockTradeService,
) -> RecipeValidationReport {
    let Some(plan) = plan else {
        return report;
    };
    for step in &plan.steps {
        let RecipeAction::Trade { sell, buy_token } = &step.action;
        let sell_token = match sell {
            AssetRef::Wallet { token, .. } | AssetRef::NodeOutput { token, .. } => *token,
        };
        if !trade.supports_pair_shape(sell_token, *buy_token) {
            report.diagnostics.push(RecipeDiagnostic::error(
                RecipeDiagnosticCode::UnsupportedTradePair,
                RecipeDiagnosticLocation::Field {
                    node_id: step.node_id,
                    field: "buyToken".to_owned(),
                },
                "trade supports only settlement token and bStock combinations",
            ));
        }
    }
    RecipeValidationReport::new(report.diagnostics)
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use alloy_primitives::{Address, B256, Bytes, U256};
    use async_trait::async_trait;
    use rust_decimal::Decimal;

    use super::*;
    use crate::{
        bstock_market::{
            AssetKind, BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockSummary,
            CatalogError,
        },
        bstock_trade::{
            PreparedApproval, PreparedTrade, PreparedTransaction, SubmitOrderRequest,
            SubmittedOrder, TokenAmount, TradeExecutionMode, TradeGateway, TradeGatewayError,
            TradeOrderStatus, TradePolicy, TradeQuote, TradeTransactionStatus,
        },
        recipes::{
            RecipeAssetTransition, RecipeCompletedNode, RecipeContext, RecipeEdge, RecipeNode,
            RecipeNodeOutput, RecipeOptionSource, RecipePortRef, RecipePrimitive,
        },
        wallet_portfolio::WalletAddress,
    };

    const SOURCE_ID: Uuid = Uuid::from_u128(1);
    const FIRST_TRADE_ID: Uuid = Uuid::from_u128(2);
    const SECOND_TRADE_ID: Uuid = Uuid::from_u128(3);
    const SETTLEMENT: Address = Address::with_last_byte(1);
    const BSTOCK: Address = Address::with_last_byte(2);
    const WRONG_TOKEN: Address = Address::with_last_byte(3);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct RecordedTrade {
        sell_token: Address,
        buy_token: Address,
        sell_amount: U256,
    }

    #[derive(Default)]
    struct RecordingGateway {
        quoted: Mutex<Vec<RecordedTrade>>,
        prepared: Mutex<Vec<RecordedTrade>>,
    }

    impl RecordingGateway {
        fn quoted(&self) -> Vec<RecordedTrade> {
            self.quoted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }

        fn prepared(&self) -> Vec<RecordedTrade> {
            self.prepared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl TradeGateway for RecordingGateway {
        async fn quotes(
            &self,
            request: &TradeQuoteRequest,
        ) -> Result<Vec<TradeQuote>, TradeGatewayError> {
            self.quoted
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(recorded(request));
            Ok(vec![TradeQuote {
                route_quote_id: format!("quote-{}", request.sell_amount),
                venue: "test".to_owned(),
                execution_mode: TradeExecutionMode::Swap,
                sell: TokenAmount {
                    token: request.sell_token,
                    amount: request.sell_amount,
                },
                buy: TokenAmount {
                    token: request.buy_token,
                    amount: request.sell_amount,
                },
                price_impact_percent: Some(Decimal::ZERO),
                approval_target: Some(Address::with_last_byte(9)),
            }])
        }

        async fn prepare_approval(
            &self,
            request: &ApprovalRequest,
        ) -> Result<PreparedApproval, TradeGatewayError> {
            Ok(PreparedApproval {
                to: request.token,
                spender: Address::with_last_byte(9),
                data: Bytes::new(),
                gas_limit: None,
                gas_price: None,
            })
        }

        async fn prepare_trade(
            &self,
            request: &PrepareTradeRequest,
            _slippage_percent: Decimal,
        ) -> Result<PreparedTrade, TradeGatewayError> {
            self.prepared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(recorded(&request.trade));
            Ok(PreparedTrade::Swap {
                transaction: PreparedTransaction {
                    to: Address::with_last_byte(9),
                    data: Bytes::new(),
                    value: U256::ZERO,
                    gas_limit: U256::from(1),
                    gas_price: U256::from(1),
                    max_priority_fee_per_gas: None,
                    min_receive_amount: U256::from(1),
                },
            })
        }

        async fn submit_order(
            &self,
            _request: &SubmitOrderRequest,
        ) -> Result<SubmittedOrder, TradeGatewayError> {
            panic!("order submission is not used by recipe preparation")
        }

        async fn order_status(
            &self,
            _order_id: &str,
        ) -> Result<TradeOrderStatus, TradeGatewayError> {
            panic!("order status is not used by recipe preparation")
        }

        async fn transaction_status(
            &self,
            _transaction_hash: B256,
        ) -> Result<TradeTransactionStatus, TradeGatewayError> {
            panic!("transaction status is not used by recipe preparation")
        }
    }

    struct FixedCatalog(Vec<BstockSummary>);

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(self.0.clone())
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn catalog_describes_trade_compatibility_with_asset_groups() {
        let (service, _) = service();

        let catalog = service
            .catalog()
            .await
            .unwrap_or_else(|error| panic!("catalog failed: {error}"));

        assert_eq!(
            catalog.trade_policy.allowed_transitions,
            vec![
                RecipeAssetTransition {
                    sell: RecipeOptionSource::SettlementTokens,
                    buy: RecipeOptionSource::Bstocks,
                },
                RecipeAssetTransition {
                    sell: RecipeOptionSource::Bstocks,
                    buy: RecipeOptionSource::SettlementTokens,
                },
            ]
        );
    }

    #[tokio::test]
    async fn rejects_an_unsupported_trade_transition_before_requesting_a_quote() {
        let (service, gateway) = service();
        let mut recipe = recipe();
        recipe.nodes.truncate(2);
        recipe.edges.truncate(1);
        recipe.nodes[1].primitive = RecipePrimitive::Trade {
            buy_token: SETTLEMENT,
        };

        let report = service.validate(&recipe);
        assert!(!report.valid);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RecipeDiagnosticCode::UnsupportedTradePair
                && diagnostic.location
                    == RecipeDiagnosticLocation::Field {
                        node_id: FIRST_TRADE_ID,
                        field: "buyToken".to_owned(),
                    }
        }));

        assert!(matches!(
            service
                .preview(&RecipePreviewRequest {
                    recipe,
                    context: context(Vec::new()),
                })
                .await,
            Err(RecipeError::InvalidRecipe(_))
        ));
        assert!(gateway.quoted().is_empty());
    }

    #[tokio::test]
    async fn previews_only_the_initial_ready_frontier() {
        let (service, gateway) = service();

        let preview = service
            .preview(&preview_request(Vec::new()))
            .await
            .unwrap_or_else(|error| panic!("preview failed: {error}"));

        assert_eq!(
            states(&preview),
            vec![RecipeNodeState::Ready, RecipeNodeState::WaitingForUpstream]
        );
        assert_eq!(
            gateway.quoted(),
            vec![RecordedTrade {
                sell_token: SETTLEMENT,
                buy_token: BSTOCK,
                sell_amount: U256::from(6),
            }]
        );
    }

    #[tokio::test]
    async fn exact_completed_output_unlocks_only_the_dependent_trade() {
        let (service, gateway) = service();

        let preview = service
            .preview(&preview_request(vec![completed_first(Some((BSTOCK, 17)))]))
            .await
            .unwrap_or_else(|error| panic!("preview failed: {error}"));

        assert_eq!(
            states(&preview),
            vec![RecipeNodeState::Complete, RecipeNodeState::Ready]
        );
        assert_eq!(
            gateway.quoted(),
            vec![RecordedTrade {
                sell_token: BSTOCK,
                buy_token: SETTLEMENT,
                sell_amount: U256::from(17),
            }]
        );
    }

    #[tokio::test]
    async fn completed_trade_without_exact_output_blocks_its_dependent() {
        let (service, gateway) = service();

        let preview = service
            .preview(&preview_request(vec![completed_first(None)]))
            .await
            .unwrap_or_else(|error| panic!("preview failed: {error}"));

        assert_eq!(
            states(&preview),
            vec![
                RecipeNodeState::Complete,
                RecipeNodeState::OutputUnavailable
            ]
        );
        assert!(gateway.quoted().is_empty());
    }

    #[tokio::test]
    async fn rejects_runtime_outputs_that_do_not_match_the_compiled_plan() {
        let (service, gateway) = service();

        for completed_nodes in [
            vec![completed_first(Some((WRONG_TOKEN, 17)))],
            vec![completed_first(Some((BSTOCK, 0)))],
            vec![
                completed_first(Some((BSTOCK, 17))),
                completed_first(Some((BSTOCK, 17))),
            ],
            vec![RecipeCompletedNode {
                node_id: Uuid::from_u128(99),
                outputs: Vec::new(),
            }],
            vec![RecipeCompletedNode {
                node_id: SECOND_TRADE_ID,
                outputs: Vec::new(),
            }],
        ] {
            assert!(matches!(
                service.preview(&preview_request(completed_nodes)).await,
                Err(RecipeError::InvalidContext)
            ));
        }
        assert!(gateway.quoted().is_empty());
    }

    #[tokio::test]
    async fn prepares_only_the_ready_frontier() {
        let (service, gateway) = service();
        let request = PrepareRecipeRequest {
            recipe: recipe(),
            context: context(vec![completed_first(Some((BSTOCK, 17)))]),
            selections: vec![selection(SECOND_TRADE_ID, 17)],
        };

        let prepared = service
            .prepare(&request)
            .await
            .unwrap_or_else(|error| panic!("preparation failed: {error}"));

        assert_eq!(prepared.steps.len(), 2);
        assert!(prepared.steps.iter().all(|step| match step {
            PreparedRecipeStep::Approval { node_id, .. }
            | PreparedRecipeStep::Trade { node_id, .. } => *node_id == SECOND_TRADE_ID,
        }));
        assert_eq!(
            gateway.prepared(),
            vec![RecordedTrade {
                sell_token: BSTOCK,
                buy_token: SETTLEMENT,
                sell_amount: U256::from(17),
            }]
        );
    }

    #[tokio::test]
    async fn prepares_the_initial_frontier_while_a_later_trade_waits() {
        let (service, gateway) = service();
        let request = PrepareRecipeRequest {
            recipe: recipe(),
            context: context(Vec::new()),
            selections: vec![selection(FIRST_TRADE_ID, 6)],
        };

        let prepared = service
            .prepare(&request)
            .await
            .unwrap_or_else(|error| panic!("preparation failed: {error}"));

        assert_eq!(prepared.steps.len(), 2);
        assert_eq!(
            gateway.prepared(),
            vec![RecordedTrade {
                sell_token: SETTLEMENT,
                buy_token: BSTOCK,
                sell_amount: U256::from(6),
            }]
        );
    }

    fn service() -> (RecipeService, Arc<RecordingGateway>) {
        let catalog = Arc::new(FixedCatalog(vec![summary()]));
        let gateway = Arc::new(RecordingGateway::default());
        let policy = TradePolicy::new([SETTLEMENT], 100, 50)
            .unwrap_or_else(|error| panic!("test policy is invalid: {error}"));
        let trade = Arc::new(BstockTradeService::new(
            gateway.clone(),
            catalog.clone(),
            policy,
        ));
        let market = Arc::new(BstockMarketService::new(catalog));
        (RecipeService::new(market, trade, vec![SETTLEMENT]), gateway)
    }

    fn preview_request(completed_nodes: Vec<RecipeCompletedNode>) -> RecipePreviewRequest {
        RecipePreviewRequest {
            recipe: recipe(),
            context: context(completed_nodes),
        }
    }

    fn context(completed_nodes: Vec<RecipeCompletedNode>) -> RecipeContext {
        RecipeContext {
            wallet: WalletAddress::try_from(Address::with_last_byte(4))
                .unwrap_or_else(|error| panic!("test wallet is invalid: {error}")),
            completed_nodes,
        }
    }

    fn completed_first(output: Option<(Address, u64)>) -> RecipeCompletedNode {
        RecipeCompletedNode {
            node_id: FIRST_TRADE_ID,
            outputs: output
                .map(|(token, amount)| RecipeNodeOutput {
                    port: "buy".to_owned(),
                    asset: TokenAmount {
                        token,
                        amount: U256::from(amount),
                    },
                })
                .into_iter()
                .collect(),
        }
    }

    fn selection(node_id: Uuid, amount: u64) -> RecipeNodeSelection {
        RecipeNodeSelection::Trade {
            node_id,
            route_quote_id: format!("quote-{amount}"),
            venue: "test".to_owned(),
        }
    }

    fn recipe() -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: SOURCE_ID,
                    primitive: RecipePrimitive::WalletAsset {
                        token: SETTLEMENT,
                        raw_amount: U256::from(6),
                    },
                },
                RecipeNode {
                    id: FIRST_TRADE_ID,
                    primitive: RecipePrimitive::Trade { buy_token: BSTOCK },
                },
                RecipeNode {
                    id: SECOND_TRADE_ID,
                    primitive: RecipePrimitive::Trade {
                        buy_token: SETTLEMENT,
                    },
                },
            ],
            edges: vec![
                RecipeEdge {
                    id: Uuid::from_u128(4),
                    from: RecipePortRef {
                        node_id: SOURCE_ID,
                        port: "asset".to_owned(),
                    },
                    to: RecipePortRef {
                        node_id: FIRST_TRADE_ID,
                        port: "sell".to_owned(),
                    },
                },
                RecipeEdge {
                    id: Uuid::from_u128(5),
                    from: RecipePortRef {
                        node_id: FIRST_TRADE_ID,
                        port: "buy".to_owned(),
                    },
                    to: RecipePortRef {
                        node_id: SECOND_TRADE_ID,
                        port: "sell".to_owned(),
                    },
                },
            ],
        }
    }

    fn states(preview: &RecipePreview) -> Vec<RecipeNodeState> {
        preview
            .nodes
            .iter()
            .map(|node| match node {
                RecipeNodePreview::Trade { state, .. } => *state,
            })
            .collect()
    }

    fn recorded(request: &TradeQuoteRequest) -> RecordedTrade {
        RecordedTrade {
            sell_token: request.sell_token,
            buy_token: request.buy_token,
            sell_amount: request.sell_amount,
        }
    }

    fn summary() -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: BSTOCK,
            },
            token_name: "Tesla bStock".to_owned(),
            token_symbol: "TSLAB".to_owned(),
            token_logo_url: None,
            token_decimals: 18,
            underlying_ticker: "TSLA".to_owned(),
            underlying_name: "Tesla".to_owned(),
            asset_kind: AssetKind::Stock,
            token_to_share_ratio: Decimal::ONE,
            token_price_usd: None,
            reference_price_usd: None,
            volume_24h_usd: None,
            market_cap_usd: None,
            market_status: None,
            tags: Vec::new(),
        }
    }
}
