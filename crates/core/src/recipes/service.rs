//! Live recipe preview and preparation.
//!
//! The compiler stops at a logical plan. This service is the boundary that may
//! call external systems to obtain quotes and prepared wallet actions.
//!
//! Preview and prepare are separate on purpose. Prepare passes the selected
//! provider quote ID back to the provider, which decides whether it is still
//! valid while building the wallet action.

use std::{collections::HashMap, sync::Arc};

use alloy_primitives::{Address, U256};
use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;

use super::{
    AssetRef, BorrowedPositionRef, LogicalPlan, PrepareRecipeRequest, PreparedRecipe,
    PreparedRecipeStep, RecipeAction, RecipeAssetTransition, RecipeBorrowPolicy, RecipeBorrowRule,
    RecipeCatalog, RecipeCompletedNode, RecipeDefinition, RecipeDiagnostic, RecipeDiagnosticCode,
    RecipeDiagnosticLocation, RecipeEarnDepositPolicy, RecipeNodePreview, RecipeNodeSelection,
    RecipeNodeState, RecipeOptionSource, RecipePreview, RecipePreviewRequest, RecipePrimitiveKind,
    RecipeRepayPolicy, RecipeRepayRule, RecipeSupplyPolicy, RecipeSupplyRule, RecipeTradePolicy,
    RecipeValidationReport, RecipeWithdrawalPolicy, RecipeWithdrawalRule,
    RecipeWithdrawalUnavailableReason, SCHEMA_VERSION, SuppliedPositionRef, compile,
};
use crate::{
    baskets::{BasketService, ListBasketsQuery},
    bstock_borrowing::{
        BorrowCapability, BorrowStage, BorrowingAsset, BorrowingProviderError, BorrowingService,
        BorrowingServiceError, BorrowingTransactionKind, BorrowingVenue, PositionAmount,
        PrepareBorrowRequest, PrepareCollateralRequest, PrepareRepaymentRequest,
        PrepareSupplyRequest, PrepareWithdrawalRequest, RepaymentCapability,
    },
    bstock_market::{BstockMarketError, BstockMarketService, ListBstocksQuery, MAX_LIST_LIMIT},
    bstock_trade::{
        ApprovalRequest, BstockTradeService, PrepareTradeRequest, SettlementTokenMetadata,
        TokenAmount, TradeError, TradeQuoteRequest,
    },
    carry::{
        CarryOpportunityQuery, CarryPosition, CarryQualification, CarryService, CarryServiceError,
        CarryTransactionKind, CarryWithdrawalAmount, PrepareCarryDepositRequest,
        PrepareCarryWithdrawalRequest,
    },
    wallet_portfolio::WalletAddress,
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
    #[error(transparent)]
    Borrowing(#[from] BorrowingServiceError),
    #[error(transparent)]
    Carry(#[from] CarryServiceError),
}

pub struct RecipeService {
    market: Arc<BstockMarketService>,
    trade: Arc<BstockTradeService>,
    baskets: Arc<BasketService>,
    borrowing: Arc<BorrowingService>,
    carry: Arc<CarryService>,
    settlement_tokens: Vec<Address>,
    settlement_decimals: HashMap<Address, u32>,
    pool: Address,
    withdrawal_policy: RecipeWithdrawalPolicy,
    borrow_policy: RecipeBorrowPolicy,
    repay_policy: RecipeRepayPolicy,
}

impl RecipeService {
    #[must_use]
    pub fn new(
        market: Arc<BstockMarketService>,
        trade: Arc<BstockTradeService>,
        baskets: Arc<BasketService>,
        borrowing: Arc<BorrowingService>,
        carry: Arc<CarryService>,
        settlement_tokens: Vec<SettlementTokenMetadata>,
        withdrawal_pool: Address,
    ) -> Self {
        let settlement_decimals = settlement_tokens
            .iter()
            .map(|token| (token.address, u32::from(token.decimals)))
            .collect();
        let settlement_tokens = settlement_tokens
            .into_iter()
            .map(|token| token.address)
            .collect::<Vec<_>>();
        let borrowing_pool_rule = RecipeWithdrawalRule {
            venue: BorrowingVenue::Venus,
            pool: withdrawal_pool,
        };
        let borrow_policy = RecipeBorrowPolicy {
            allowed_inputs: vec![RecipeBorrowRule {
                venue: BorrowingVenue::Venus,
                pool: withdrawal_pool,
                input: RecipeOptionSource::SuppliedPositions,
            }],
            debt_tokens: borrowing.supported_debt_tokens().to_vec(),
            minimum_health_factor: borrowing.minimum_health_factor(),
        };
        let repay_policy = RecipeRepayPolicy {
            allowed_inputs: vec![RecipeRepayRule {
                venue: BorrowingVenue::Venus,
                pool: withdrawal_pool,
                input: RecipeOptionSource::BorrowedPositions,
            }],
            debt_tokens: borrowing.supported_debt_tokens().to_vec(),
        };
        Self {
            market,
            trade,
            baskets,
            borrowing,
            carry,
            settlement_tokens,
            settlement_decimals,
            pool: withdrawal_pool,
            withdrawal_policy: RecipeWithdrawalPolicy {
                allowed_inputs: vec![borrowing_pool_rule],
            },
            borrow_policy,
            repay_policy,
        }
    }

    /// The asset moves a trade may make, which the builder turns into its token
    /// pickers and the model is told about when it drafts a recipe.
    fn trade_transitions(&self) -> Vec<RecipeAssetTransition> {
        let mut transitions = vec![
            RecipeAssetTransition {
                sell: RecipeOptionSource::SettlementTokens,
                buy: RecipeOptionSource::Bstocks,
            },
            RecipeAssetTransition {
                sell: RecipeOptionSource::Bstocks,
                buy: RecipeOptionSource::SettlementTokens,
            },
        ];
        if self.trade.offers_settlement_pairs() {
            transitions.push(RecipeAssetTransition {
                sell: RecipeOptionSource::SettlementTokens,
                buy: RecipeOptionSource::SettlementTokens,
            });
        }
        // A DTF is a third thing a stablecoin buys, traded at its own venue.
        transitions.push(RecipeAssetTransition {
            sell: RecipeOptionSource::SettlementTokens,
            buy: RecipeOptionSource::Baskets,
        });
        transitions.push(RecipeAssetTransition {
            sell: RecipeOptionSource::Baskets,
            buy: RecipeOptionSource::SettlementTokens,
        });
        transitions
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
        let baskets = self
            .baskets
            .list(&ListBasketsQuery {
                search: None,
                category: None,
                limit: MAX_LIST_LIMIT,
            })
            .await
            .unwrap_or_default();
        Ok(RecipeCatalog {
            schema_version: SCHEMA_VERSION,
            primitives: RecipePrimitiveKind::ALL
                .into_iter()
                .map(RecipePrimitiveKind::descriptor)
                .collect(),
            settlement_tokens: self.settlement_tokens.clone(),
            bstocks,
            baskets,
            trade_policy: RecipeTradePolicy {
                allowed_transitions: self.trade_transitions(),
            },
            supply_policy: RecipeSupplyPolicy {
                allowed_inputs: vec![RecipeSupplyRule {
                    venue: BorrowingVenue::Venus,
                    input: RecipeOptionSource::Bstocks,
                }],
            },
            withdrawal_policy: self.withdrawal_policy.clone(),
            borrow_policy: self.borrow_policy.clone(),
            repay_policy: self.repay_policy.clone(),
            earn_deposit_policy: RecipeEarnDepositPolicy {
                allowed_inputs: vec![
                    RecipePrimitiveKind::WalletAsset,
                    RecipePrimitiveKind::Borrow,
                ],
                deposit_tokens: self.settlement_tokens.clone(),
            },
        })
    }

    /// Runs the pure compiler without contacting a provider.
    #[must_use]
    pub fn validate(&self, recipe: &RecipeDefinition) -> RecipeValidationReport {
        let compiled = compile(recipe, self.pool);
        validate_action_policy(
            compiled.report,
            compiled.plan.as_ref(),
            &self.trade,
            &self.settlement_tokens,
            &self.withdrawal_policy,
            &self.borrow_policy,
            &self.repay_policy,
        )
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
        let plan = compile_plan(
            &request.recipe,
            &self.trade,
            &self.settlement_tokens,
            &self.withdrawal_policy,
            &self.borrow_policy,
            &self.repay_policy,
            self.pool,
        )?;
        let completed = validate_completed_context(&plan, &request.context.completed_nodes)?;
        let mut nodes = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            if completed.contains_key(&step.node_id) {
                nodes.push(completed_preview(step));
                continue;
            }

            let preview = match &step.action {
                RecipeAction::Trade { sell, buy_token } => {
                    self.preview_trade(step, sell, *buy_token, &completed, request.context.wallet)
                        .await?
                }
                RecipeAction::Supply { asset, .. } => {
                    self.preview_supply(step, asset, &completed).await?
                }
                RecipeAction::Withdraw { position } => {
                    self.preview_withdrawal(step, *position, &completed, request.context.wallet)
                        .await?
                }
                RecipeAction::Borrow {
                    collateral,
                    debt_token,
                    raw_amount,
                } => {
                    self.preview_borrow(
                        step,
                        *collateral,
                        *debt_token,
                        *raw_amount,
                        &completed,
                        request.context.wallet,
                    )
                    .await?
                }
                RecipeAction::Repay { debt } => {
                    self.preview_repayment(step.node_id, *debt, request.context.wallet)
                        .await?
                }
                RecipeAction::EarnDeposit {
                    asset,
                    collateral_token,
                    investment_id,
                } => {
                    self.preview_earn_deposit(
                        step,
                        asset,
                        *collateral_token,
                        investment_id,
                        &completed,
                    )
                    .await?
                }
                RecipeAction::EarnWithdraw { position } => {
                    self.preview_earn_withdrawal(
                        step.node_id,
                        &position.investment_id,
                        request.context.wallet,
                    )
                    .await?
                }
            };
            nodes.push(preview);
        }
        Ok(RecipePreview { nodes })
    }

    /// Whether a trade's non-settlement side is a DTF rather than a bStock.
    ///
    /// The two venues answer the same questions with the same types, so the
    /// only thing that differs is which catalog the asset came from.
    async fn is_basket_trade(&self, sell_token: Address, buy_token: Address) -> bool {
        let Ok(baskets) = self
            .baskets
            .list(&ListBasketsQuery {
                search: None,
                category: None,
                limit: MAX_LIST_LIMIT,
            })
            .await
        else {
            return false;
        };
        baskets
            .iter()
            .any(|basket| basket.id.contract == buy_token || basket.id.contract == sell_token)
    }

    async fn preview_trade(
        &self,
        step: &super::LogicalStep,
        sell_asset: &AssetRef,
        buy_token: Address,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
        wallet: WalletAddress,
    ) -> Result<RecipeNodePreview, RecipeError> {
        let (state, routes, waiting_for) = match resolve_asset(sell_asset, completed) {
            ResolvedAsset::Ready(asset) => {
                let request = TradeQuoteRequest {
                    wallet,
                    sell_token: asset.token,
                    buy_token,
                    sell_amount: asset.amount,
                };
                let routes = if self.is_basket_trade(asset.token, buy_token).await {
                    self.baskets.quotes(&request).await?
                } else {
                    self.trade.quotes(&request).await?
                };
                (RecipeNodeState::Ready, routes, Vec::new())
            }
            ResolvedAsset::Waiting => (
                RecipeNodeState::WaitingForUpstream,
                Vec::new(),
                step.dependencies.clone(),
            ),
            ResolvedAsset::Unavailable => (
                RecipeNodeState::OutputUnavailable,
                Vec::new(),
                step.dependencies.clone(),
            ),
        };
        Ok(RecipeNodePreview::Trade {
            node_id: step.node_id,
            state,
            routes,
            waiting_for,
        })
    }

    async fn preview_supply(
        &self,
        step: &super::LogicalStep,
        asset: &AssetRef,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
    ) -> Result<RecipeNodePreview, RecipeError> {
        let (state, market, waiting_for) = match resolve_asset(asset, completed) {
            ResolvedAsset::Ready(asset) => {
                let market = self.borrowing.supply_market(asset.token).await?;
                let state = market
                    .as_ref()
                    .map_or(RecipeNodeState::Unavailable, |market| {
                        if market.supply_available {
                            RecipeNodeState::Ready
                        } else {
                            RecipeNodeState::Unavailable
                        }
                    });
                (state, market, Vec::new())
            }
            ResolvedAsset::Waiting => (
                RecipeNodeState::WaitingForUpstream,
                None,
                step.dependencies.clone(),
            ),
            ResolvedAsset::Unavailable => (
                RecipeNodeState::OutputUnavailable,
                None,
                step.dependencies.clone(),
            ),
        };
        Ok(RecipeNodePreview::Supply {
            node_id: step.node_id,
            state,
            market,
            waiting_for,
        })
    }

    async fn preview_earn_deposit(
        &self,
        step: &super::LogicalStep,
        asset: &AssetRef,
        collateral_token: Address,
        investment_id: &str,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
    ) -> Result<RecipeNodePreview, RecipeError> {
        let (state, opportunity, waiting_for) = match resolve_asset(asset, completed) {
            ResolvedAsset::Ready(asset) => {
                // One flaky provider call should not fail the whole preview, so a
                // route that cannot be read reads the same as one that is gone.
                let opportunity = self
                    .carry
                    .opportunities(CarryOpportunityQuery {
                        collateral_token,
                        debt_token: asset.token,
                    })
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .find(|opportunity| opportunity.investment_id == investment_id);
                // An unqualified route is still returned so the editor can show
                // why it cannot be used instead of silently clearing it.
                let state =
                    opportunity
                        .as_ref()
                        .map_or(RecipeNodeState::Unavailable, |opportunity| {
                            if opportunity.qualification == CarryQualification::Eligible {
                                RecipeNodeState::Ready
                            } else {
                                RecipeNodeState::Unavailable
                            }
                        });
                (state, opportunity, Vec::new())
            }
            ResolvedAsset::Waiting => (
                RecipeNodeState::WaitingForUpstream,
                None,
                step.dependencies.clone(),
            ),
            ResolvedAsset::Unavailable => (
                RecipeNodeState::OutputUnavailable,
                None,
                step.dependencies.clone(),
            ),
        };
        Ok(RecipeNodePreview::EarnDeposit {
            node_id: step.node_id,
            state,
            opportunity,
            waiting_for,
        })
    }

    async fn preview_withdrawal(
        &self,
        step: &super::LogicalStep,
        position: SuppliedPositionRef,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
        wallet: WalletAddress,
    ) -> Result<RecipeNodePreview, RecipeError> {
        if waits_for_upstream(step, completed) {
            return Ok(RecipeNodePreview::Withdraw {
                node_id: step.node_id,
                state: RecipeNodeState::WaitingForUpstream,
                position: None,
                unavailable_reason: None,
                waiting_for: step.dependencies.clone(),
            });
        }
        let node_id = step.node_id;
        Ok(match self.resolve_withdrawal(wallet, position).await? {
            ResolvedWithdrawal::Ready(position) => RecipeNodePreview::Withdraw {
                node_id,
                state: RecipeNodeState::Ready,
                position: Some(position),
                unavailable_reason: None,
                waiting_for: Vec::new(),
            },
            ResolvedWithdrawal::Unavailable { position, reason } => RecipeNodePreview::Withdraw {
                node_id,
                state: RecipeNodeState::Unavailable,
                position,
                unavailable_reason: Some(reason),
                waiting_for: Vec::new(),
            },
        })
    }

    async fn preview_borrow(
        &self,
        step: &super::LogicalStep,
        collateral: SuppliedPositionRef,
        debt_token: Address,
        raw_amount: U256,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
        wallet: WalletAddress,
    ) -> Result<RecipeNodePreview, RecipeError> {
        if waits_for_upstream(step, completed) {
            return Ok(RecipeNodePreview::Borrow {
                node_id: step.node_id,
                state: RecipeNodeState::WaitingForUpstream,
                capability: None,
                waiting_for: step.dependencies.clone(),
            });
        }
        let node_id = step.node_id;
        let capability = self
            .borrowing
            .borrow_capability(
                wallet,
                collateral.pool,
                collateral.token,
                debt_token,
                raw_amount,
            )
            .await?;
        let state = if capability.unavailable_reasons.is_empty() {
            RecipeNodeState::Ready
        } else {
            RecipeNodeState::Unavailable
        };
        Ok(RecipeNodePreview::Borrow {
            node_id,
            state,
            capability: Some(capability),
            waiting_for: Vec::new(),
        })
    }

    async fn preview_repayment(
        &self,
        node_id: Uuid,
        debt: BorrowedPositionRef,
        wallet: WalletAddress,
    ) -> Result<RecipeNodePreview, RecipeError> {
        let capability = self
            .borrowing
            .repayment_capability(wallet, debt.pool, debt.token)
            .await?;
        let state = if capability.unavailable_reasons.is_empty() {
            RecipeNodeState::Ready
        } else {
            RecipeNodeState::Unavailable
        };
        Ok(RecipeNodePreview::Repay {
            node_id,
            state,
            capability: Some(capability),
            waiting_for: Vec::new(),
        })
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
        let plan = compile_plan(
            &request.recipe,
            &self.trade,
            &self.settlement_tokens,
            &self.withdrawal_policy,
            &self.borrow_policy,
            &self.repay_policy,
            self.pool,
        )?;
        let completed = validate_completed_context(&plan, &request.context.completed_nodes)?;
        let ready = self
            .ready_steps(&plan, &completed, request.context.wallet)
            .await?;
        if ready.is_empty() {
            return Err(RecipeError::StepNotReady);
        }
        let selections = selections_by_node(&request.selections)?;
        ensure_every_ready_trade_is_selected(&ready, &selections)?;

        let mut prepared = Vec::with_capacity(ready.len() * 2);
        for (step, input) in ready {
            prepared.extend(
                self.prepare_ready_step(step, input, &selections, request.context.wallet)
                    .await?,
            );
        }
        Ok(PreparedRecipe { steps: prepared })
    }

    /// Turns one ready step into the transactions its action needs.
    async fn prepare_ready_step(
        &self,
        step: &super::LogicalStep,
        input: ReadyInput,
        selections: &HashMap<Uuid, &RecipeNodeSelection>,
        wallet: WalletAddress,
    ) -> Result<Vec<PreparedRecipeStep>, RecipeError> {
        let mut prepared = Vec::new();
        match (&step.action, input) {
            (RecipeAction::Trade { buy_token, .. }, ReadyInput::Asset(input)) => {
                let selection = selections
                    .get(&step.node_id)
                    .copied()
                    .ok_or(RecipeError::InvalidSelection)?;
                prepared.extend(
                    self.prepare_trade_step(step.node_id, input, *buy_token, selection, wallet)
                        .await?,
                );
            }
            (RecipeAction::Supply { .. }, ReadyInput::Asset(input)) => {
                prepared.push(
                    self.prepare_supply_step(step.node_id, input, wallet)
                        .await?,
                );
            }
            (RecipeAction::Withdraw { .. }, ReadyInput::Withdrawal(position)) => {
                prepared.push(
                    self.prepare_withdrawal_step(step.node_id, position, wallet)
                        .await?,
                );
            }
            (RecipeAction::Borrow { .. }, ReadyInput::Borrow(capability, collateral)) => {
                prepared.push(
                    self.prepare_borrow_step(step.node_id, *capability, collateral, wallet)
                        .await?,
                );
            }
            (RecipeAction::Repay { .. }, ReadyInput::Repay(capability)) => {
                prepared.push(
                    self.prepare_repayment_step(step.node_id, *capability, wallet)
                        .await?,
                );
            }
            (RecipeAction::EarnWithdraw { position }, ReadyInput::EarnWithdrawal) => {
                prepared.push(
                    self.prepare_earn_withdrawal_step(
                        step.node_id,
                        &position.investment_id,
                        wallet,
                    )
                    .await?,
                );
            }
            (
                RecipeAction::EarnDeposit {
                    collateral_token,
                    investment_id,
                    ..
                },
                ReadyInput::Asset(input),
            ) => {
                prepared.push(
                    self.prepare_earn_deposit_step(
                        step.node_id,
                        input,
                        *collateral_token,
                        investment_id.clone(),
                        wallet,
                    )
                    .await?,
                );
            }
            _ => return Err(RecipeError::InvalidContext),
        }
        Ok(prepared)
    }

    async fn ready_steps<'a>(
        &self,
        plan: &'a LogicalPlan,
        completed: &HashMap<Uuid, &RecipeCompletedNode>,
        wallet: WalletAddress,
    ) -> Result<Vec<(&'a super::LogicalStep, ReadyInput)>, RecipeError> {
        let mut ready = Vec::new();
        for step in &plan.steps {
            if completed.contains_key(&step.node_id) {
                continue;
            }
            match &step.action {
                RecipeAction::Trade { sell, .. }
                | RecipeAction::Supply { asset: sell, .. }
                | RecipeAction::EarnDeposit { asset: sell, .. } => {
                    if let ResolvedAsset::Ready(asset) = resolve_asset(sell, completed) {
                        ready.push((step, ReadyInput::Asset(asset)));
                    }
                }
                RecipeAction::Withdraw { position } => {
                    if let ResolvedWithdrawal::Ready(position) =
                        self.resolve_withdrawal(wallet, *position).await?
                    {
                        ready.push((step, ReadyInput::Withdrawal(position)));
                    }
                }
                RecipeAction::Borrow {
                    collateral,
                    debt_token,
                    raw_amount,
                } => {
                    let capability = self
                        .borrowing
                        .borrow_capability(
                            wallet,
                            collateral.pool,
                            collateral.token,
                            *debt_token,
                            *raw_amount,
                        )
                        .await?;
                    if capability.unavailable_reasons.is_empty() {
                        ready.push((step, ReadyInput::Borrow(Box::new(capability), *collateral)));
                    }
                }
                RecipeAction::Repay { debt } => {
                    let capability = self
                        .borrowing
                        .repayment_capability(wallet, debt.pool, debt.token)
                        .await?;
                    if capability.unavailable_reasons.is_empty() {
                        ready.push((step, ReadyInput::Repay(Box::new(capability))));
                    }
                }
                RecipeAction::EarnWithdraw { position } => {
                    if self
                        .earn_position(wallet, &position.investment_id)
                        .await?
                        .is_some()
                    {
                        ready.push((step, ReadyInput::EarnWithdrawal));
                    }
                }
            }
        }
        Ok(ready)
    }

    async fn prepare_trade_step(
        &self,
        node_id: Uuid,
        input: TokenAmount,
        buy_token: Address,
        selection: &RecipeNodeSelection,
        wallet: WalletAddress,
    ) -> Result<[PreparedRecipeStep; 2], RecipeError> {
        let RecipeNodeSelection::Trade {
            route_quote_id,
            venue,
            ..
        } = selection;
        let trade = TradeQuoteRequest {
            wallet,
            sell_token: input.token,
            buy_token,
            sell_amount: input.amount,
        };
        let approval_request = ApprovalRequest {
            token: input.token,
            amount: input.amount,
            venue: venue.clone(),
        };
        let trade_request = PrepareTradeRequest {
            trade,
            route_quote_id: route_quote_id.clone(),
        };
        // An RFQ order is submitted and polled where it was quoted, so the
        // prepared step names the venue family that will carry it.
        let basket = self.is_basket_trade(input.token, buy_token).await;
        let (approval, trade) = if basket {
            (
                self.baskets.prepare_approval(&approval_request).await?,
                self.baskets.prepare_trade(&trade_request).await?,
            )
        } else {
            (
                self.trade.prepare_approval(&approval_request).await?,
                self.trade.prepare_trade(&trade_request).await?,
            )
        };
        Ok([
            PreparedRecipeStep::Approval {
                node_id,
                action: approval,
            },
            if basket {
                PreparedRecipeStep::BasketTrade {
                    node_id,
                    action: trade,
                }
            } else {
                PreparedRecipeStep::Trade {
                    node_id,
                    action: trade,
                }
            },
        ])
    }

    async fn prepare_supply_step(
        &self,
        node_id: Uuid,
        asset: TokenAmount,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let market = self
            .borrowing
            .supply_market(asset.token)
            .await?
            .filter(|market| market.supply_available)
            .ok_or_else(rejected_by_provider)?;
        let amount = decimal_from_raw(asset.amount, market.token_decimals)?;
        let action = self
            .borrowing
            .prepare_supply(&PrepareSupplyRequest {
                wallet,
                collateral_token: asset.token,
                amount,
            })
            .await?;
        Ok(PreparedRecipeStep::Supply { node_id, action })
    }

    async fn prepare_earn_deposit_step(
        &self,
        node_id: Uuid,
        asset: TokenAmount,
        collateral_token: Address,
        investment_id: String,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let decimals = self
            .settlement_decimals
            .get(&asset.token)
            .copied()
            .ok_or_else(invalid_provider_data)?;
        let amount = decimal_from_raw(asset.amount, decimals)?;
        let action = self
            .carry
            .prepare_deposit(&PrepareCarryDepositRequest {
                wallet,
                collateral_token,
                investment_id,
                amount,
            })
            .await?;
        let kinds = action
            .transactions
            .iter()
            .map(|transaction| transaction.kind)
            .collect::<Vec<_>>();
        if !matches!(
            kinds.as_slice(),
            [CarryTransactionKind::Deposit]
                | [CarryTransactionKind::Approve, CarryTransactionKind::Deposit]
        ) {
            return Err(invalid_provider_data());
        }
        Ok(PreparedRecipeStep::EarnDeposit {
            node_id,
            action: Box::new(action),
        })
    }

    async fn prepare_borrow_step(
        &self,
        node_id: Uuid,
        capability: BorrowCapability,
        collateral: SuppliedPositionRef,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        if capability.stage == BorrowStage::EnableCollateral {
            let action = self
                .borrowing
                .prepare_collateral(&PrepareCollateralRequest {
                    wallet,
                    collateral_token: collateral.token,
                })
                .await?;
            if action.transactions.is_empty() {
                let refreshed = self
                    .borrowing
                    .borrow_capability(
                        wallet,
                        collateral.pool,
                        collateral.token,
                        capability.debt.contract,
                        capability.requested_raw_amount,
                    )
                    .await?;
                if refreshed.stage == BorrowStage::Borrow
                    && refreshed.unavailable_reasons.is_empty()
                {
                    return self
                        .prepare_borrow_transaction(node_id, refreshed, wallet)
                        .await;
                }
                return Err(RecipeError::StepNotReady);
            }
            if action.transactions.len() != 1
                || action.transactions[0].kind != BorrowingTransactionKind::EnableCollateral
            {
                return Err(invalid_provider_data());
            }
            return Ok(PreparedRecipeStep::EnableCollateral { node_id, action });
        }
        self.prepare_borrow_transaction(node_id, capability, wallet)
            .await
    }

    async fn prepare_borrow_transaction(
        &self,
        node_id: Uuid,
        capability: BorrowCapability,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let action = self
            .borrowing
            .prepare_borrow(&PrepareBorrowRequest {
                wallet,
                debt_token: capability.debt.contract,
                amount: capability.requested_amount,
            })
            .await?;
        if action.transactions.len() != 1
            || action.transactions[0].kind != BorrowingTransactionKind::Borrow
        {
            return Err(invalid_provider_data());
        }
        Ok(PreparedRecipeStep::Borrow {
            node_id,
            action,
            output: TokenAmount {
                token: capability.debt.contract,
                amount: capability.requested_raw_amount,
            },
        })
    }

    async fn prepare_withdrawal_step(
        &self,
        node_id: Uuid,
        position: BorrowingAsset,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let action = self
            .borrowing
            .prepare_withdrawal(&PrepareWithdrawalRequest {
                wallet,
                collateral_token: position.token.contract,
                amount: PositionAmount::Full,
            })
            .await?;
        Ok(PreparedRecipeStep::Withdraw { node_id, action })
    }

    async fn prepare_repayment_step(
        &self,
        node_id: Uuid,
        capability: RepaymentCapability,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let action = self
            .borrowing
            .prepare_full_repayment(&PrepareRepaymentRequest {
                wallet,
                debt_token: capability.debt.contract,
                amount: PositionAmount::Full,
            })
            .await?;
        let kinds = action
            .transactions
            .iter()
            .map(|transaction| transaction.kind)
            .collect::<Vec<_>>();
        if kinds != [BorrowingTransactionKind::Repay]
            && kinds
                != [
                    BorrowingTransactionKind::Approve,
                    BorrowingTransactionKind::Repay,
                ]
        {
            return Err(invalid_provider_data());
        }
        Ok(PreparedRecipeStep::Repay { node_id, action })
    }

    /// Reads the wallet's live earn position for one investment.
    async fn earn_position(
        &self,
        wallet: WalletAddress,
        investment_id: &str,
    ) -> Result<Option<CarryPosition>, RecipeError> {
        Ok(self
            .carry
            .positions(wallet)
            .await?
            .positions
            .into_iter()
            .find(|position| {
                position
                    .investment_ids
                    .iter()
                    .any(|held| held == investment_id)
            })
            // A withdrawn position leaves dust behind, and dust is not something
            // the provider will move; offering it is offering a failure.
            .filter(|position| position.value_usd > Decimal::ZERO))
    }

    async fn preview_earn_withdrawal(
        &self,
        node_id: Uuid,
        investment_id: &str,
        wallet: WalletAddress,
    ) -> Result<RecipeNodePreview, RecipeError> {
        let position = self.earn_position(wallet, investment_id).await?;
        Ok(RecipeNodePreview::EarnWithdraw {
            node_id,
            state: if position.is_some() {
                RecipeNodeState::Ready
            } else {
                RecipeNodeState::OutputUnavailable
            },
            position,
            waiting_for: Vec::new(),
        })
    }

    async fn prepare_earn_withdrawal_step(
        &self,
        node_id: Uuid,
        investment_id: &str,
        wallet: WalletAddress,
    ) -> Result<PreparedRecipeStep, RecipeError> {
        let action = self
            .carry
            .prepare_withdrawal(&PrepareCarryWithdrawalRequest {
                wallet,
                investment_id: investment_id.to_owned(),
                amount: CarryWithdrawalAmount::Full,
            })
            .await?;
        Ok(PreparedRecipeStep::EarnWithdraw { node_id, action })
    }

    async fn resolve_withdrawal(
        &self,
        wallet: WalletAddress,
        position: SuppliedPositionRef,
    ) -> Result<ResolvedWithdrawal, RecipeError> {
        if !self
            .withdrawal_policy
            .allowed_inputs
            .iter()
            .any(|rule| rule.venue == position.venue && rule.pool == position.pool)
        {
            return Ok(ResolvedWithdrawal::Unavailable {
                position: None,
                reason: RecipeWithdrawalUnavailableReason::UnsupportedPool,
            });
        }

        let supported = match self.borrowing.supply_market(position.token).await {
            Ok(Some(market)) => market.venue == position.venue,
            Ok(None) | Err(BorrowingServiceError::UnsupportedCollateral) => false,
            Err(error) => return Err(error.into()),
        };
        if !supported {
            return Ok(ResolvedWithdrawal::Unavailable {
                position: None,
                reason: RecipeWithdrawalUnavailableReason::PositionNotFound,
            });
        }

        let account = self.borrowing.account(wallet).await?;
        let supplied = account
            .positions
            .iter()
            .find(|candidate| candidate.pool == position.pool)
            .and_then(|candidate| {
                candidate
                    .supplied
                    .iter()
                    .find(|asset| asset.token.contract == position.token)
            })
            .cloned();
        let Some(supplied) = supplied else {
            return Ok(ResolvedWithdrawal::Unavailable {
                position: None,
                reason: RecipeWithdrawalUnavailableReason::PositionNotFound,
            });
        };
        if account.total_borrowed_usd > Decimal::ZERO {
            return Ok(ResolvedWithdrawal::Unavailable {
                position: Some(supplied),
                reason: RecipeWithdrawalUnavailableReason::OutstandingDebt,
            });
        }
        Ok(ResolvedWithdrawal::Ready(supplied))
    }
}

fn completed_preview(step: &super::LogicalStep) -> RecipeNodePreview {
    match &step.action {
        RecipeAction::Trade { .. } => RecipeNodePreview::Trade {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            routes: Vec::new(),
            waiting_for: Vec::new(),
        },
        RecipeAction::Supply { .. } => RecipeNodePreview::Supply {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            market: None,
            waiting_for: Vec::new(),
        },
        RecipeAction::EarnDeposit { .. } => RecipeNodePreview::EarnDeposit {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            opportunity: None,
            waiting_for: Vec::new(),
        },
        RecipeAction::EarnWithdraw { .. } => RecipeNodePreview::EarnWithdraw {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            position: None,
            waiting_for: Vec::new(),
        },
        RecipeAction::Withdraw { .. } => RecipeNodePreview::Withdraw {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            position: None,
            unavailable_reason: None,
            waiting_for: Vec::new(),
        },
        RecipeAction::Borrow { .. } => RecipeNodePreview::Borrow {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            capability: None,
            waiting_for: Vec::new(),
        },
        RecipeAction::Repay { .. } => RecipeNodePreview::Repay {
            node_id: step.node_id,
            state: RecipeNodeState::Complete,
            capability: None,
            waiting_for: Vec::new(),
        },
    }
}

fn decimal_from_raw(raw: U256, decimals: u32) -> Result<Decimal, RecipeError> {
    let digits = raw.to_string();
    let scale = usize::try_from(decimals).map_err(|_| invalid_provider_data())?;
    let value = if scale == 0 {
        digits
    } else if digits.len() <= scale {
        format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
    } else {
        let decimal_at = digits.len() - scale;
        format!("{}.{}", &digits[..decimal_at], &digits[decimal_at..])
    };
    value.parse().map_err(|_| invalid_provider_data())
}

fn ensure_every_ready_trade_is_selected(
    ready: &[(&super::LogicalStep, ReadyInput)],
    selections: &HashMap<Uuid, &RecipeNodeSelection>,
) -> Result<(), RecipeError> {
    let ready_trades = ready
        .iter()
        .filter(|(step, _)| matches!(step.action, RecipeAction::Trade { .. }))
        .count();
    if selections.len() != ready_trades
        || ready.iter().any(|(step, _)| {
            matches!(step.action, RecipeAction::Trade { .. })
                && !selections.contains_key(&step.node_id)
        })
    {
        return Err(RecipeError::InvalidSelection);
    }
    Ok(())
}

fn withdrawal_diagnostics(
    node_id: Uuid,
    position: &SuppliedPositionRef,
    withdrawal_policy: &RecipeWithdrawalPolicy,
    settlement_tokens: &[Address],
) -> Vec<RecipeDiagnostic> {
    let mut diagnostics = Vec::new();
    if !withdrawal_policy
        .allowed_inputs
        .iter()
        .any(|rule| rule.venue == position.venue && rule.pool == position.pool)
    {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedWithdrawalPosition,
            RecipeDiagnosticLocation::Field {
                node_id,
                field: "pool".to_owned(),
            },
            "withdraw supports only configured lending pools",
        ));
    }
    if settlement_tokens.contains(&position.token) {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedWithdrawalPosition,
            RecipeDiagnosticLocation::Field {
                node_id,
                field: "token".to_owned(),
            },
            "withdraw accepts supported supplied bStock positions",
        ));
    }
    diagnostics
}

fn earn_deposit_diagnostics(
    node_id: Uuid,
    deposit_token: Address,
    collateral_token: Address,
    settlement_tokens: &[Address],
) -> Vec<RecipeDiagnostic> {
    let mut diagnostics = Vec::new();
    if !settlement_tokens.contains(&deposit_token) {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedEarnDepositAsset,
            RecipeDiagnosticLocation::Port {
                node_id,
                port: "asset".to_owned(),
            },
            "an earn deposit accepts supported settlement tokens",
        ));
    }
    if settlement_tokens.contains(&collateral_token) {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedEarnCollateral,
            RecipeDiagnosticLocation::Field {
                node_id,
                field: "collateralToken".to_owned(),
            },
            "the benchmark collateral must be a supported bStock",
        ));
    }
    diagnostics
}

fn invalid_provider_data() -> RecipeError {
    BorrowingServiceError::Provider(BorrowingProviderError::InvalidData).into()
}

fn rejected_by_provider() -> RecipeError {
    BorrowingServiceError::Provider(BorrowingProviderError::Rejected).into()
}

#[derive(Clone, Copy)]
enum ResolvedAsset {
    Ready(TokenAmount),
    Waiting,
    Unavailable,
}

enum ReadyInput {
    Asset(TokenAmount),
    Withdrawal(BorrowingAsset),
    EarnWithdrawal,
    Borrow(Box<BorrowCapability>, SuppliedPositionRef),
    Repay(Box<RepaymentCapability>),
}

enum ResolvedWithdrawal {
    Ready(BorrowingAsset),
    Unavailable {
        position: Option<BorrowingAsset>,
        reason: RecipeWithdrawalUnavailableReason,
    },
}

/// True while an earlier step in this run has yet to create what this one
/// consumes. A position a `Supply` step is about to open does not exist on
/// chain yet, so asking the venue about it would report it missing.
fn waits_for_upstream(
    step: &super::LogicalStep,
    completed: &HashMap<Uuid, &RecipeCompletedNode>,
) -> bool {
    step.dependencies
        .iter()
        .any(|dependency| !completed.contains_key(dependency))
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

        match &step.action {
            RecipeAction::Trade { buy_token, .. } => {
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
            RecipeAction::Supply { .. }
            | RecipeAction::Withdraw { .. }
            | RecipeAction::Repay { .. }
            | RecipeAction::EarnDeposit { .. }
            | RecipeAction::EarnWithdraw { .. }
                if !node.outputs.is_empty() =>
            {
                return Err(RecipeError::InvalidContext);
            }
            RecipeAction::Supply { .. }
            | RecipeAction::Withdraw { .. }
            | RecipeAction::Repay { .. }
            | RecipeAction::EarnDeposit { .. }
            | RecipeAction::EarnWithdraw { .. } => {}
            RecipeAction::Borrow {
                debt_token,
                raw_amount,
                ..
            } => {
                if node.outputs.len() != 1
                    || node.outputs[0].port != "borrowed"
                    || node.outputs[0].asset.token != *debt_token
                    || node.outputs[0].asset.amount != *raw_amount
                {
                    return Err(RecipeError::InvalidContext);
                }
            }
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
        let input = match &step.action {
            RecipeAction::Trade { sell, .. } => sell,
            RecipeAction::Supply { asset, .. } | RecipeAction::EarnDeposit { asset, .. } => asset,
            RecipeAction::Withdraw { .. }
            | RecipeAction::Borrow { .. }
            | RecipeAction::Repay { .. }
            | RecipeAction::EarnWithdraw { .. } => continue,
        };
        if let AssetRef::NodeOutput { node_id, port, .. } = input {
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
    settlement_tokens: &[Address],
    withdrawal_policy: &RecipeWithdrawalPolicy,
    borrow_policy: &RecipeBorrowPolicy,
    repay_policy: &RecipeRepayPolicy,
    pool: Address,
) -> Result<LogicalPlan, RecipeError> {
    let compiled = compile(recipe, pool);
    match compiled.plan {
        Some(plan) => {
            let report = validate_action_policy(
                compiled.report,
                Some(&plan),
                trade,
                settlement_tokens,
                withdrawal_policy,
                borrow_policy,
                repay_policy,
            );
            if report.valid {
                Ok(plan)
            } else {
                Err(RecipeError::InvalidRecipe(report))
            }
        }
        None => Err(RecipeError::InvalidRecipe(compiled.report)),
    }
}

fn validate_action_policy(
    mut report: RecipeValidationReport,
    plan: Option<&LogicalPlan>,
    trade: &BstockTradeService,
    settlement_tokens: &[Address],
    withdrawal_policy: &RecipeWithdrawalPolicy,
    borrow_policy: &RecipeBorrowPolicy,
    repay_policy: &RecipeRepayPolicy,
) -> RecipeValidationReport {
    let Some(plan) = plan else {
        return report;
    };
    for step in &plan.steps {
        match &step.action {
            RecipeAction::Trade { sell, buy_token } => {
                let sell_token = asset_token(sell);
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
            RecipeAction::Supply { asset, .. }
                if settlement_tokens.contains(&asset_token(asset)) =>
            {
                report.diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::UnsupportedSupplyAsset,
                    RecipeDiagnosticLocation::Port {
                        node_id: step.node_id,
                        port: "asset".to_owned(),
                    },
                    "supply accepts supported bStocks, not settlement tokens",
                ));
            }
            RecipeAction::Supply { .. } | RecipeAction::EarnWithdraw { .. } => {}
            RecipeAction::EarnDeposit {
                asset,
                collateral_token,
                ..
            } => report.diagnostics.extend(earn_deposit_diagnostics(
                step.node_id,
                asset_token(asset),
                *collateral_token,
                settlement_tokens,
            )),
            RecipeAction::Withdraw { position } => {
                report.diagnostics.extend(withdrawal_diagnostics(
                    step.node_id,
                    position,
                    withdrawal_policy,
                    settlement_tokens,
                ));
            }
            RecipeAction::Borrow {
                collateral,
                debt_token,
                ..
            } => {
                if !borrow_policy
                    .allowed_inputs
                    .iter()
                    .any(|rule| rule.venue == collateral.venue && rule.pool == collateral.pool)
                    || settlement_tokens.contains(&collateral.token)
                {
                    report.diagnostics.push(RecipeDiagnostic::error(
                        RecipeDiagnosticCode::UnsupportedBorrowPosition,
                        RecipeDiagnosticLocation::Port {
                            node_id: step.node_id,
                            port: "collateral".to_owned(),
                        },
                        "borrow accepts a supported supplied bStock position",
                    ));
                }
                if !borrow_policy.debt_tokens.contains(debt_token)
                    || !trade.supports_pair_shape(collateral.token, *debt_token)
                {
                    report.diagnostics.push(RecipeDiagnostic::error(
                        RecipeDiagnosticCode::UnsupportedBorrowAsset,
                        RecipeDiagnosticLocation::Field {
                            node_id: step.node_id,
                            field: "debtToken".to_owned(),
                        },
                        "borrow supports only configured Venus debt assets",
                    ));
                }
            }
            RecipeAction::Repay { debt } => {
                if let Some(diagnostic) = validate_repay_policy(step.node_id, *debt, repay_policy) {
                    report.diagnostics.push(diagnostic);
                }
            }
        }
    }
    RecipeValidationReport::new(report.diagnostics)
}

fn validate_repay_policy(
    node_id: Uuid,
    debt: BorrowedPositionRef,
    policy: &RecipeRepayPolicy,
) -> Option<RecipeDiagnostic> {
    let supported_position = policy.allowed_inputs.iter().any(|rule| {
        rule.venue == debt.venue
            && rule.pool == debt.pool
            && rule.input == RecipeOptionSource::BorrowedPositions
    });
    if supported_position && policy.debt_tokens.contains(&debt.token) {
        return None;
    }
    Some(RecipeDiagnostic::error(
        RecipeDiagnosticCode::UnsupportedRepaymentPosition,
        RecipeDiagnosticLocation::Port {
            node_id,
            port: "debt".to_owned(),
        },
        "repay accepts only configured Venus debt positions",
    ))
}

fn asset_token(asset: &AssetRef) -> Address {
    match asset {
        AssetRef::Wallet { token, .. } | AssetRef::NodeOutput { token, .. } => *token,
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use alloy_primitives::{Address, B256, Bytes, U256};
    use async_trait::async_trait;
    use rust_decimal::Decimal;

    use super::*;
    use crate::carry::{
        CarryAccount, CarryActionPlan, CarryActionPreview, CarryPolicy, EarnActionGateway,
        EarnDepositCommand, EarnOpportunity, EarnProviderError, EarnRateType, EarnReader,
        EarnRedemptionClaimCommand, EarnWithdrawalCommand, PreparedCarryTransaction,
    };
    use crate::{
        bstock_borrowing::{
            BorrowCapability, BorrowCapabilityRequest, BorrowStage, BorrowingAccount,
            BorrowingActionGateway, BorrowingAsset, BorrowingPolicy, BorrowingPosition,
            BorrowingPreview, BorrowingReader, BorrowingToken, BorrowingTransactionKind,
            BorrowingTransactionState, BorrowingTransactionStatus, PositionAmount,
            PrepareBorrowRequest, PrepareCollateralRequest, PrepareRepaymentRequest,
            PrepareWithdrawalRequest, PreparedBorrowingAction, PreparedBorrowingTransaction,
            RepaymentCapabilityRequest, RepaymentCapabilityUnavailableReason, SupplyMarket,
            SupplyMarketUnavailableReason,
        },
        bstock_market::{
            AssetKind, BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockSummary,
            CatalogError,
        },
        bstock_trade::SettlementPairs,
        bstock_trade::{
            PreparedApproval, PreparedTrade, PreparedTransaction, SubmitOrderRequest,
            SubmittedOrder, TokenAmount, TradeExecutionMode, TradeGateway, TradeGatewayError,
            TradeOrderStatus, TradePolicy, TradeQuote, TradeTransactionStatus,
        },
        recipes::{
            RecipeAssetTransition, RecipeCompletedNode, RecipeContext, RecipeEdge, RecipeNode,
            RecipeNodeOutput, RecipeOptionSource, RecipePortRef, RecipePrimitive,
            RecipeWithdrawalRule, RecipeWithdrawalUnavailableReason,
        },
        wallet_portfolio::WalletAddress,
    };

    const SOURCE_ID: Uuid = Uuid::from_u128(1);
    const FIRST_TRADE_ID: Uuid = Uuid::from_u128(2);
    const SECOND_TRADE_ID: Uuid = Uuid::from_u128(3);
    const SUPPLY_ID: Uuid = Uuid::from_u128(6);
    const POSITION_ID: Uuid = Uuid::from_u128(8);
    const WITHDRAW_ID: Uuid = Uuid::from_u128(9);
    const BORROW_ID: Uuid = Uuid::from_u128(10);
    const DEBT_ID: Uuid = Uuid::from_u128(11);
    const REPAY_ID: Uuid = Uuid::from_u128(12);
    const SETTLEMENT: Address = Address::with_last_byte(1);
    const BSTOCK: Address = Address::with_last_byte(2);
    const WRONG_TOKEN: Address = Address::with_last_byte(3);
    const POOL: Address = Address::with_last_byte(8);
    const INVESTMENT_ID: &str = "investment-1";
    const EARN_ID: Uuid = Uuid::from_u128(13);

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

    struct TestBorrowing;

    #[async_trait]
    impl BorrowingReader for TestBorrowing {
        async fn read_borrow_capability(
            &self,
            request: &BorrowCapabilityRequest,
        ) -> Result<BorrowCapability, BorrowingProviderError> {
            let collateral_enabled =
                request.raw_amount != U256::from(2_000_000_000_000_000_000_u128);
            Ok(BorrowCapability {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                pool: request.pool,
                collateral: BorrowingToken {
                    contract: request.collateral_token,
                    symbol: "TSLAB".to_owned(),
                },
                debt: BorrowingToken {
                    contract: request.debt_token,
                    symbol: "USDT".to_owned(),
                },
                requested_amount: Decimal::ONE,
                requested_raw_amount: request.raw_amount,
                debt_token_decimals: 18,
                conservative_max_amount: Decimal::new(10, 0),
                available_borrow_liquidity: Decimal::new(100, 0),
                borrow_apy_percent: Decimal::new(5, 0),
                health_factor_before: None,
                health_factor_after: Some(Decimal::new(2, 0)),
                collateral_enabled,
                stage: if collateral_enabled {
                    BorrowStage::Borrow
                } else {
                    BorrowStage::EnableCollateral
                },
                unavailable_reasons: Vec::new(),
                observed_at_ms: 1,
            })
        }

        async fn read_repayment_capability(
            &self,
            request: &RepaymentCapabilityRequest,
        ) -> Result<RepaymentCapability, BorrowingProviderError> {
            let unfunded_wallet = WalletAddress::try_from(Address::with_last_byte(99))
                .unwrap_or_else(|error| panic!("test wallet rejected: {error}"));
            let funded = request.wallet != unfunded_wallet;
            Ok(RepaymentCapability {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                pool: request.pool,
                debt: BorrowingToken {
                    contract: request.debt_token,
                    symbol: "USDT".to_owned(),
                },
                debt_amount: Decimal::new(3, 0),
                wallet_balance: if funded {
                    Decimal::new(4, 0)
                } else {
                    Decimal::new(2, 0)
                },
                approval_amount: Decimal::new(31, 1),
                approval_required: true,
                unavailable_reasons: if funded {
                    Vec::new()
                } else {
                    vec![RepaymentCapabilityUnavailableReason::InsufficientWalletBalance]
                },
                observed_at_ms: 1,
            })
        }

        async fn read_supply_market(
            &self,
            token: Address,
        ) -> Result<Option<SupplyMarket>, BorrowingProviderError> {
            Ok((token == BSTOCK).then_some(SupplyMarket {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                token: crate::bstock_borrowing::BorrowingToken {
                    contract: BSTOCK,
                    symbol: "TSLAB".to_owned(),
                },
                market: Address::with_last_byte(8),
                token_decimals: 18,
                supply_apy_percent: Decimal::ONE,
                remaining_supply_capacity: None,
                supply_available: true,
                unavailable_reasons: Vec::<SupplyMarketUnavailableReason>::new(),
                observed_at_ms: 1,
            }))
        }

        async fn read_markets(
            &self,
            collateral_assets: &[BstockId],
            debt_tokens: &[Address],
        ) -> Result<Vec<crate::bstock_borrowing::BorrowingMarket>, BorrowingProviderError> {
            Ok(collateral_assets
                .iter()
                .flat_map(|collateral| {
                    debt_tokens
                        .iter()
                        .map(|debt| borrowing_market(collateral.contract, *debt))
                })
                .collect())
        }

        async fn read_account(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
    }

    #[async_trait]
    impl BorrowingActionGateway for TestBorrowing {
        async fn prepare_supply(
            &self,
            _request: &PrepareSupplyRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_collateral(
            &self,
            _request: &PrepareCollateralRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Ok(PreparedBorrowingAction {
                transactions: vec![PreparedBorrowingTransaction {
                    kind: BorrowingTransactionKind::EnableCollateral,
                    to: Address::with_last_byte(8),
                    data: Bytes::new(),
                    value: U256::ZERO,
                    gas_limit: None,
                    gas_price: None,
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                }],
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_borrow(
            &self,
            _request: &PrepareBorrowRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Ok(PreparedBorrowingAction {
                transactions: vec![PreparedBorrowingTransaction {
                    kind: BorrowingTransactionKind::Borrow,
                    to: Address::with_last_byte(7),
                    data: Bytes::new(),
                    value: U256::ZERO,
                    gas_limit: None,
                    gas_price: None,
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                }],
                preview: BorrowingPreview {
                    health_factor_after: Some(Decimal::new(2, 0)),
                    ..BorrowingPreview::default()
                },
            })
        }

        async fn prepare_repayment_approval(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("repayment preparation is not used by recipes")
        }

        async fn prepare_repayment(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("repayment preparation is not used by recipes")
        }

        async fn prepare_full_repayment(
            &self,
            request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            assert_eq!(request.amount, PositionAmount::Full);
            Ok(PreparedBorrowingAction {
                transactions: vec![
                    borrowing_transaction(BorrowingTransactionKind::Approve),
                    borrowing_transaction(BorrowingTransactionKind::Repay),
                ],
                preview: BorrowingPreview::default(),
            })
        }

        async fn prepare_withdrawal(
            &self,
            _request: &PrepareWithdrawalRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("withdrawal preparation is not used by recipes")
        }

        async fn transaction_status(
            &self,
            _transaction_hash: B256,
        ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
            Ok(BorrowingTransactionStatus {
                state: BorrowingTransactionState::Unknown,
                block_number: None,
                executed_at_ms: None,
            })
        }
    }

    struct WithdrawalBorrowing {
        account: BorrowingAccount,
        withdrawals: Mutex<Vec<(WalletAddress, Address, PositionAmount)>>,
    }

    impl WithdrawalBorrowing {
        fn withdrawals(&self) -> Vec<(WalletAddress, Address, PositionAmount)> {
            self.withdrawals
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl BorrowingReader for WithdrawalBorrowing {
        async fn read_borrow_capability(
            &self,
            _request: &BorrowCapabilityRequest,
        ) -> Result<BorrowCapability, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn read_repayment_capability(
            &self,
            _request: &RepaymentCapabilityRequest,
        ) -> Result<RepaymentCapability, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }

        async fn read_supply_market(
            &self,
            token: Address,
        ) -> Result<Option<SupplyMarket>, BorrowingProviderError> {
            Ok((token == BSTOCK).then_some(SupplyMarket {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                token: BorrowingToken {
                    contract: BSTOCK,
                    symbol: "TSLAB".to_owned(),
                },
                market: Address::with_last_byte(7),
                token_decimals: 18,
                supply_apy_percent: Decimal::ONE,
                remaining_supply_capacity: None,
                supply_available: true,
                unavailable_reasons: Vec::new(),
                observed_at_ms: 1,
            }))
        }

        async fn read_markets(
            &self,
            _collateral_assets: &[BstockId],
            _debt_tokens: &[Address],
        ) -> Result<Vec<crate::bstock_borrowing::BorrowingMarket>, BorrowingProviderError> {
            Ok(Vec::new())
        }

        async fn read_account(
            &self,
            _wallet: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingProviderError> {
            Ok(self.account.clone())
        }
    }

    #[async_trait]
    impl BorrowingActionGateway for WithdrawalBorrowing {
        async fn prepare_supply(
            &self,
            _request: &PrepareSupplyRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("supply preparation is not used by withdrawal recipes")
        }

        async fn prepare_collateral(
            &self,
            _request: &PrepareCollateralRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("collateral preparation is not used by withdrawal recipes")
        }

        async fn prepare_borrow(
            &self,
            _request: &PrepareBorrowRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("borrow preparation is not used by withdrawal recipes")
        }

        async fn prepare_repayment_approval(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("repayment approval is not used by withdrawal recipes")
        }

        async fn prepare_repayment(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("repayment preparation is not used by withdrawal recipes")
        }

        async fn prepare_full_repayment(
            &self,
            _request: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            unreachable!("full repayment preparation is not used by withdrawal recipes")
        }

        async fn prepare_withdrawal(
            &self,
            request: &PrepareWithdrawalRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            self.withdrawals
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((
                    request.wallet,
                    request.collateral_token,
                    request.amount.clone(),
                ));
            Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            })
        }

        async fn transaction_status(
            &self,
            _transaction_hash: B256,
        ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
            unreachable!("transaction status is not used by withdrawal recipes")
        }
    }

    #[tokio::test]
    async fn settlement_pairs_reach_the_catalog_only_when_the_deployment_allows_them() {
        // The builder's token pickers and the model's grammar both read this, so
        // the gate has to show up here rather than only inside the trade policy.
        let settlement_to_settlement = RecipeAssetTransition {
            sell: RecipeOptionSource::SettlementTokens,
            buy: RecipeOptionSource::SettlementTokens,
        };

        let (closed, _) = gated_service(SettlementPairs::Rejected);
        let closed = closed
            .catalog()
            .await
            .unwrap_or_else(|error| panic!("catalog failed: {error}"));
        let (open, _) = gated_service(SettlementPairs::Allowed);
        let open = open
            .catalog()
            .await
            .unwrap_or_else(|error| panic!("catalog failed: {error}"));

        assert!(
            !closed
                .trade_policy
                .allowed_transitions
                .contains(&settlement_to_settlement)
        );
        assert!(
            open.trade_policy
                .allowed_transitions
                .contains(&settlement_to_settlement)
        );
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
                RecipeAssetTransition {
                    sell: RecipeOptionSource::SettlementTokens,
                    buy: RecipeOptionSource::Baskets,
                },
                RecipeAssetTransition {
                    sell: RecipeOptionSource::Baskets,
                    buy: RecipeOptionSource::SettlementTokens,
                },
            ]
        );
        assert_eq!(
            catalog.supply_policy.allowed_inputs,
            vec![RecipeSupplyRule {
                venue: BorrowingVenue::Venus,
                input: RecipeOptionSource::Bstocks,
            }]
        );
        assert_eq!(
            catalog.withdrawal_policy.allowed_inputs,
            vec![RecipeWithdrawalRule {
                venue: BorrowingVenue::Venus,
                pool: POOL,
            }]
        );
        assert_eq!(catalog.borrow_policy.debt_tokens, vec![SETTLEMENT]);
        assert_eq!(catalog.repay_policy.debt_tokens, vec![SETTLEMENT]);
        assert_eq!(
            catalog.repay_policy.allowed_inputs,
            vec![RecipeRepayRule {
                venue: BorrowingVenue::Venus,
                pool: POOL,
                input: RecipeOptionSource::BorrowedPositions,
            }]
        );
        assert_eq!(
            catalog.borrow_policy.minimum_health_factor,
            Decimal::new(15, 1)
        );
    }

    #[tokio::test]
    async fn borrow_preview_and_prepare_preserve_the_exact_runtime_output() {
        let (service, _) = service();
        let raw_amount = U256::from(1_000_000_000_000_000_000_u128);
        let recipe = borrow_recipe(raw_amount);
        let preview = service
            .preview(&RecipePreviewRequest {
                recipe: recipe.clone(),
                context: context(Vec::new()),
            })
            .await
            .unwrap_or_else(|error| panic!("borrow preview failed: {error}"));
        assert!(matches!(
            preview.nodes.as_slice(),
            [RecipeNodePreview::Borrow {
                state: RecipeNodeState::Ready,
                capability: Some(BorrowCapability {
                    stage: BorrowStage::Borrow,
                    ..
                }),
                ..
            }]
        ));

        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe,
                context: context(Vec::new()),
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("borrow preparation failed: {error}"));
        assert!(matches!(
            prepared.steps.as_slice(),
            [PreparedRecipeStep::Borrow {
                node_id,
                output: TokenAmount { token, amount },
                ..
            }] if *node_id == BORROW_ID && *token == SETTLEMENT && *amount == raw_amount
        ));
    }

    #[tokio::test]
    async fn borrow_prepares_only_collateral_activation_when_membership_is_missing() {
        let (service, _) = service();
        let raw_amount = U256::from(2_000_000_000_000_000_000_u128);
        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe: borrow_recipe(raw_amount),
                context: context(Vec::new()),
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("collateral activation failed: {error}"));

        assert!(matches!(
            prepared.steps.as_slice(),
            [PreparedRecipeStep::EnableCollateral { node_id, action }]
                if *node_id == BORROW_ID
                    && action.transactions.len() == 1
                    && action.transactions[0].kind == BorrowingTransactionKind::EnableCollateral
        ));
    }

    #[tokio::test]
    async fn repayment_preview_reports_an_unfunded_debt_position() {
        let (service, _) = service();
        let wallet = WalletAddress::try_from(Address::with_last_byte(99))
            .unwrap_or_else(|error| panic!("test wallet is invalid: {error}"));
        let preview = service
            .preview(&RecipePreviewRequest {
                recipe: repayment_recipe(POOL, SETTLEMENT),
                context: RecipeContext {
                    wallet,
                    completed_nodes: Vec::new(),
                },
            })
            .await
            .unwrap_or_else(|error| panic!("repayment preview failed: {error}"));

        assert!(matches!(
            preview.nodes.as_slice(),
            [RecipeNodePreview::Repay {
                node_id,
                state: RecipeNodeState::Unavailable,
                capability: Some(RepaymentCapability {
                    unavailable_reasons,
                    ..
                }),
                ..
            }] if *node_id == REPAY_ID
                && unavailable_reasons == &[RepaymentCapabilityUnavailableReason::InsufficientWalletBalance]
        ));
    }

    #[tokio::test]
    async fn repayment_prepare_preserves_approve_then_repay_order() {
        let (service, _) = service();
        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe: repayment_recipe(POOL, SETTLEMENT),
                context: context(Vec::new()),
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("repayment preparation failed: {error}"));

        assert!(matches!(
            prepared.steps.as_slice(),
            [PreparedRecipeStep::Repay { node_id, action }]
                if *node_id == REPAY_ID
                    && action.transactions.iter().map(|transaction| transaction.kind).collect::<Vec<_>>()
                        == [BorrowingTransactionKind::Approve, BorrowingTransactionKind::Repay]
        ));
    }

    #[tokio::test]
    async fn completed_repayment_rejects_runtime_outputs() {
        let (service, _) = service();
        let completed = RecipeCompletedNode {
            node_id: REPAY_ID,
            outputs: vec![RecipeNodeOutput {
                port: "unexpected".to_owned(),
                asset: TokenAmount {
                    token: SETTLEMENT,
                    amount: U256::ONE,
                },
            }],
        };

        assert!(matches!(
            service
                .preview(&RecipePreviewRequest {
                    recipe: repayment_recipe(POOL, SETTLEMENT),
                    context: context(vec![completed]),
                })
                .await,
            Err(RecipeError::InvalidContext)
        ));
    }

    #[tokio::test]
    async fn withdraw_preview_resolves_the_live_supplied_position() {
        let (service, _) = withdrawal_service(withdrawal_account(true, Decimal::ZERO), POOL);

        let preview = service
            .preview(&RecipePreviewRequest {
                recipe: withdrawal_recipe(POOL, BSTOCK),
                context: context(Vec::new()),
            })
            .await
            .unwrap_or_else(|error| panic!("withdrawal preview failed: {error}"));

        assert!(matches!(
            preview.nodes.as_slice(),
            [RecipeNodePreview::Withdraw {
                node_id,
                state: RecipeNodeState::Ready,
                position: Some(BorrowingAsset { token, amount, .. }),
                unavailable_reason: None,
                waiting_for,
            }] if *node_id == WITHDRAW_ID
                && token.contract == BSTOCK
                && *amount == Decimal::new(193, 4)
                && waiting_for.is_empty()
        ));
    }

    #[tokio::test]
    async fn withdraw_preview_reports_missing_position_and_outstanding_debt() {
        for (account, expected) in [
            (
                withdrawal_account(false, Decimal::ZERO),
                RecipeWithdrawalUnavailableReason::PositionNotFound,
            ),
            (
                withdrawal_account(true, Decimal::ONE),
                RecipeWithdrawalUnavailableReason::OutstandingDebt,
            ),
        ] {
            let (service, _) = withdrawal_service(account, POOL);
            let preview = service
                .preview(&RecipePreviewRequest {
                    recipe: withdrawal_recipe(POOL, BSTOCK),
                    context: context(Vec::new()),
                })
                .await
                .unwrap_or_else(|error| panic!("withdrawal preview failed: {error}"));

            assert!(matches!(
                preview.nodes.as_slice(),
                [RecipeNodePreview::Withdraw {
                    state: RecipeNodeState::Unavailable,
                    unavailable_reason: Some(reason),
                    ..
                }] if *reason == expected
            ));
        }
    }

    #[test]
    fn withdraw_validation_rejects_a_pool_outside_policy() {
        let (service, _) = withdrawal_service(withdrawal_account(true, Decimal::ZERO), POOL);

        let report = service.validate(&withdrawal_recipe(Address::with_last_byte(99), BSTOCK));

        assert!(!report.valid);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RecipeDiagnosticCode::UnsupportedWithdrawalPosition
                && diagnostic.location
                    == RecipeDiagnosticLocation::Field {
                        node_id: WITHDRAW_ID,
                        field: "pool".to_owned(),
                    }
        }));
    }

    #[tokio::test]
    async fn withdraw_prepare_forwards_the_full_live_position() {
        let (service, gateway) = withdrawal_service(withdrawal_account(true, Decimal::ZERO), POOL);
        let wallet = context(Vec::new()).wallet;

        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe: withdrawal_recipe(POOL, BSTOCK),
                context: RecipeContext {
                    wallet,
                    completed_nodes: Vec::new(),
                },
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("withdrawal preparation failed: {error}"));

        assert!(matches!(
            prepared.steps.as_slice(),
            [PreparedRecipeStep::Withdraw { node_id, .. }] if *node_id == WITHDRAW_ID
        ));
        assert_eq!(
            gateway.withdrawals(),
            vec![(wallet, BSTOCK, PositionAmount::Full)]
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
            | PreparedRecipeStep::Trade { node_id, .. }
            | PreparedRecipeStep::BasketTrade { node_id, .. } => *node_id == SECOND_TRADE_ID,
            PreparedRecipeStep::Supply { .. }
            | PreparedRecipeStep::Withdraw { .. }
            | PreparedRecipeStep::EnableCollateral { .. }
            | PreparedRecipeStep::Borrow { .. }
            | PreparedRecipeStep::Repay { .. }
            | PreparedRecipeStep::EarnDeposit { .. }
            | PreparedRecipeStep::EarnWithdraw { .. } => false,
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

    #[tokio::test]
    async fn prepares_a_wallet_funded_earn_deposit_as_approval_then_deposit() {
        let (service, _) = service();
        let recipe = earn_deposit_recipe(
            SETTLEMENT,
            U256::from(1_000_000_000_000_000_000_u128),
            BSTOCK,
            INVESTMENT_ID,
        );

        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe,
                context: context(Vec::new()),
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("preparation failed: {error}"));

        let [PreparedRecipeStep::EarnDeposit { node_id, action }] = prepared.steps.as_slice()
        else {
            panic!("expected a single earn deposit step");
        };
        assert_eq!(*node_id, EARN_ID);
        assert_eq!(
            action
                .transactions
                .iter()
                .map(|transaction| transaction.kind)
                .collect::<Vec<_>>(),
            vec![CarryTransactionKind::Approve, CarryTransactionKind::Deposit]
        );
        assert_eq!(action.opportunity.investment_id, INVESTMENT_ID);
    }

    #[tokio::test]
    async fn rejects_an_earn_deposit_of_a_non_settlement_token() {
        let (service, _) = service();
        let recipe = earn_deposit_recipe(BSTOCK, U256::from(6), BSTOCK, INVESTMENT_ID);

        let report = service.validate(&recipe);

        assert!(!report.valid);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RecipeDiagnosticCode::UnsupportedEarnDepositAsset
        }));
    }

    #[tokio::test]
    async fn rejects_an_earn_deposit_without_a_selected_investment() {
        let (service, _) = service();
        let recipe = earn_deposit_recipe(SETTLEMENT, U256::from(6), BSTOCK, "   ");

        let report = service.validate(&recipe);

        assert!(!report.valid);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == RecipeDiagnosticCode::MissingInvestmentId)
        );
    }

    #[tokio::test]
    async fn rejects_an_earn_deposit_benchmarked_against_unrelated_collateral() {
        let (service, _) = service();
        let recipe = borrowed_earn_deposit_recipe(WRONG_TOKEN);

        let report = service.validate(&recipe);

        assert!(!report.valid);
        assert!(
            report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == RecipeDiagnosticCode::EarnCollateralMismatch
            })
        );
    }

    #[tokio::test]
    async fn holds_a_borrow_funded_earn_deposit_until_the_borrow_settles() {
        let (service, _) = service();
        let recipe = borrowed_earn_deposit_recipe(BSTOCK);

        let preview = service
            .preview(&RecipePreviewRequest {
                recipe,
                context: context(Vec::new()),
            })
            .await
            .unwrap_or_else(|error| panic!("preview failed: {error}"));

        let deposit = preview
            .nodes
            .iter()
            .find_map(|node| match node {
                RecipeNodePreview::EarnDeposit { node_id, state, .. } if *node_id == EARN_ID => {
                    Some(*state)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the earn deposit node is missing from the preview"));
        assert_eq!(deposit, RecipeNodeState::WaitingForUpstream);
    }

    #[tokio::test]
    async fn rejects_settlement_token_supply_before_provider_io() {
        let (service, gateway) = service();
        let recipe = supply_recipe(SETTLEMENT, U256::from(6));

        let report = service.validate(&recipe);

        assert!(!report.valid);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RecipeDiagnosticCode::UnsupportedSupplyAsset
                && diagnostic.location
                    == RecipeDiagnosticLocation::Port {
                        node_id: SUPPLY_ID,
                        port: "asset".to_owned(),
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
    async fn prepares_ready_supply_without_a_route_selection() {
        let (service, _) = service();
        let recipe = supply_recipe(BSTOCK, U256::from(1_000_000_000_000_000_000_u128));

        let preview = service
            .preview(&RecipePreviewRequest {
                recipe: recipe.clone(),
                context: context(Vec::new()),
            })
            .await
            .unwrap_or_else(|error| panic!("supply preview failed: {error}"));
        assert_eq!(states(&preview), vec![RecipeNodeState::Ready]);

        let prepared = service
            .prepare(&PrepareRecipeRequest {
                recipe,
                context: context(Vec::new()),
                selections: Vec::new(),
            })
            .await
            .unwrap_or_else(|error| panic!("supply preparation failed: {error}"));
        assert!(matches!(
            prepared.steps.as_slice(),
            [PreparedRecipeStep::Supply { node_id, .. }] if *node_id == SUPPLY_ID
        ));
    }

    #[test]
    fn converts_raw_supply_amount_exactly_and_rejects_decimal_overflow() {
        assert_eq!(
            decimal_from_raw(U256::from(1_250_001_000_000_000_000_u128), 18)
                .unwrap_or_else(|error| panic!("exact amount failed: {error}")),
            Decimal::new(1_250_001, 6)
        );
        assert!(decimal_from_raw(U256::MAX, 0).is_err());
    }

    fn borrowing_market(
        collateral: Address,
        debt: Address,
    ) -> crate::bstock_borrowing::BorrowingMarket {
        crate::bstock_borrowing::BorrowingMarket {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            collateral: BorrowingToken {
                contract: collateral,
                symbol: "TSLAB".to_owned(),
            },
            collateral_market: POOL,
            debt: BorrowingToken {
                contract: debt,
                symbol: "USDT".to_owned(),
            },
            debt_market: POOL,
            collateral_factor: Decimal::new(6, 1),
            liquidation_threshold: Decimal::new(7, 1),
            remaining_supply_capacity: Decimal::new(1_000, 0),
            available_borrow_liquidity: Decimal::new(1_000, 0),
            borrow_apy_percent: Decimal::new(2, 0),
            supply_available: true,
            borrow_available: true,
            oracle_valid: true,
            unavailable_reasons: Vec::new(),
            observed_at_ms: 1,
        }
    }

    struct TestEarn;

    impl TestEarn {
        fn opportunity() -> EarnOpportunity {
            EarnOpportunity {
                investment_id: INVESTMENT_ID.to_owned(),
                investment_name: "Lista USDT".to_owned(),
                protocol_id: "lista".to_owned(),
                protocol_name: "Lista".to_owned(),
                protocol_logo_url: None,
                asset: BorrowingToken {
                    contract: SETTLEMENT,
                    symbol: "USDT".to_owned(),
                },
                rate_type: EarnRateType::Apy,
                base_rate_bps: 900,
                base_rate_display: "9%".to_owned(),
                reported_tvl_usd: None,
                investable: true,
                observed_at_ms: 1,
            }
        }

        fn plan(kinds: &[CarryTransactionKind]) -> CarryActionPlan {
            CarryActionPlan {
                transactions: kinds
                    .iter()
                    .map(|kind| PreparedCarryTransaction {
                        kind: *kind,
                        to: POOL,
                        data: Bytes::new(),
                        value: U256::ZERO,
                        gas_limit: None,
                        gas_price: None,
                        max_priority_fee_per_gas: None,
                        max_fee_per_gas: None,
                    })
                    .collect(),
                preview: CarryActionPreview::default(),
            }
        }
    }

    #[async_trait]
    impl EarnReader for TestEarn {
        async fn opportunities(
            &self,
            asset: Address,
        ) -> Result<Vec<EarnOpportunity>, EarnProviderError> {
            Ok(if asset == SETTLEMENT {
                vec![Self::opportunity()]
            } else {
                Vec::new()
            })
        }

        async fn opportunity(
            &self,
            investment_id: &str,
        ) -> Result<Option<EarnOpportunity>, EarnProviderError> {
            Ok((investment_id == INVESTMENT_ID).then(Self::opportunity))
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<CarryAccount, EarnProviderError> {
            Err(EarnProviderError::Unavailable)
        }
    }

    #[async_trait]
    impl EarnActionGateway for TestEarn {
        async fn prepare_deposit(
            &self,
            _command: &EarnDepositCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            Ok(TestEarn::plan(&[
                CarryTransactionKind::Approve,
                CarryTransactionKind::Deposit,
            ]))
        }

        async fn prepare_withdrawal(
            &self,
            _command: &EarnWithdrawalCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            Err(EarnProviderError::Unavailable)
        }

        async fn prepare_redemption_claim(
            &self,
            _command: &EarnRedemptionClaimCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            Err(EarnProviderError::Unavailable)
        }
    }

    fn carry_service(borrowing: Arc<BorrowingService>) -> Arc<CarryService> {
        let earn = Arc::new(TestEarn);
        Arc::new(CarryService::new(
            borrowing,
            earn.clone(),
            earn,
            CarryPolicy::new(vec![INVESTMENT_ID.to_owned()], 100),
        ))
    }

    fn test_baskets() -> Arc<BasketService> {
        Arc::new(BasketService::new(
            crate::baskets::test_support::no_baskets(),
            Arc::new(RecordingGateway::default()),
            TradePolicy::new([SETTLEMENT], 100, 50, SettlementPairs::Rejected)
                .unwrap_or_else(|error| panic!("test policy is invalid: {error}")),
        ))
    }

    fn service() -> (RecipeService, Arc<RecordingGateway>) {
        gated_service(SettlementPairs::Rejected)
    }

    fn gated_service(settlement_pairs: SettlementPairs) -> (RecipeService, Arc<RecordingGateway>) {
        let catalog = Arc::new(FixedCatalog(vec![summary()]));
        let gateway = Arc::new(RecordingGateway::default());
        let policy = TradePolicy::new([SETTLEMENT], 100, 50, settlement_pairs)
            .unwrap_or_else(|error| panic!("test policy is invalid: {error}"));
        let trade = Arc::new(BstockTradeService::new(
            gateway.clone(),
            catalog.clone(),
            policy,
        ));
        let market = Arc::new(BstockMarketService::new(catalog));
        let borrowing_gateway = Arc::new(TestBorrowing);
        let borrowing = Arc::new(BorrowingService::new(
            Arc::new(FixedCatalog(vec![summary()])),
            borrowing_gateway.clone(),
            borrowing_gateway,
            BorrowingPolicy::new(vec![SETTLEMENT])
                .unwrap_or_else(|error| panic!("test borrowing policy is invalid: {error}")),
        ));
        (
            RecipeService::new(
                market,
                trade,
                test_baskets(),
                borrowing.clone(),
                carry_service(borrowing),
                vec![settlement_metadata()],
                POOL,
            ),
            gateway,
        )
    }

    fn withdrawal_service(
        account: BorrowingAccount,
        withdrawal_pool: Address,
    ) -> (RecipeService, Arc<WithdrawalBorrowing>) {
        let catalog = Arc::new(FixedCatalog(vec![summary()]));
        let trade = Arc::new(BstockTradeService::new(
            Arc::new(RecordingGateway::default()),
            catalog.clone(),
            TradePolicy::new([SETTLEMENT], 100, 50, SettlementPairs::Rejected)
                .unwrap_or_else(|error| panic!("test policy is invalid: {error}")),
        ));
        let market = Arc::new(BstockMarketService::new(catalog));
        let gateway = Arc::new(WithdrawalBorrowing {
            account,
            withdrawals: Mutex::new(Vec::new()),
        });
        let borrowing = Arc::new(BorrowingService::new(
            Arc::new(FixedCatalog(vec![summary()])),
            gateway.clone(),
            gateway.clone(),
            BorrowingPolicy::new(vec![SETTLEMENT])
                .unwrap_or_else(|error| panic!("test borrowing policy is invalid: {error}")),
        ));
        (
            RecipeService::new(
                market,
                trade,
                test_baskets(),
                borrowing.clone(),
                carry_service(borrowing),
                vec![settlement_metadata()],
                withdrawal_pool,
            ),
            gateway,
        )
    }

    fn settlement_metadata() -> SettlementTokenMetadata {
        SettlementTokenMetadata {
            address: SETTLEMENT,
            symbol: "USDT".to_owned(),
            decimals: 18,
        }
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

    fn supply_recipe(token: Address, raw_amount: U256) -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: SOURCE_ID,
                    primitive: RecipePrimitive::WalletAsset { token, raw_amount },
                },
                RecipeNode {
                    id: SUPPLY_ID,
                    primitive: RecipePrimitive::Supply {
                        venue: BorrowingVenue::Venus,
                    },
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(7),
                from: RecipePortRef {
                    node_id: SOURCE_ID,
                    port: "asset".to_owned(),
                },
                to: RecipePortRef {
                    node_id: SUPPLY_ID,
                    port: "asset".to_owned(),
                },
            }],
        }
    }

    fn withdrawal_recipe(pool: Address, token: Address) -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: POSITION_ID,
                    primitive: RecipePrimitive::SuppliedPosition {
                        venue: BorrowingVenue::Venus,
                        pool,
                        token,
                    },
                },
                RecipeNode {
                    id: WITHDRAW_ID,
                    primitive: RecipePrimitive::Withdraw,
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(10),
                from: RecipePortRef {
                    node_id: POSITION_ID,
                    port: "position".to_owned(),
                },
                to: RecipePortRef {
                    node_id: WITHDRAW_ID,
                    port: "position".to_owned(),
                },
            }],
        }
    }

    fn borrow_recipe(raw_amount: U256) -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: POSITION_ID,
                    primitive: RecipePrimitive::SuppliedPosition {
                        venue: BorrowingVenue::Venus,
                        pool: POOL,
                        token: BSTOCK,
                    },
                },
                RecipeNode {
                    id: BORROW_ID,
                    primitive: RecipePrimitive::Borrow {
                        debt_token: SETTLEMENT,
                        raw_amount,
                    },
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(11),
                from: RecipePortRef {
                    node_id: POSITION_ID,
                    port: "position".to_owned(),
                },
                to: RecipePortRef {
                    node_id: BORROW_ID,
                    port: "collateral".to_owned(),
                },
            }],
        }
    }

    fn earn_deposit_recipe(
        token: Address,
        raw_amount: U256,
        collateral_token: Address,
        investment_id: &str,
    ) -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: SOURCE_ID,
                    primitive: RecipePrimitive::WalletAsset { token, raw_amount },
                },
                RecipeNode {
                    id: EARN_ID,
                    primitive: RecipePrimitive::EarnDeposit {
                        collateral_token,
                        investment_id: investment_id.to_owned(),
                    },
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(14),
                from: RecipePortRef {
                    node_id: SOURCE_ID,
                    port: "asset".to_owned(),
                },
                to: RecipePortRef {
                    node_id: EARN_ID,
                    port: "asset".to_owned(),
                },
            }],
        }
    }

    fn borrowed_earn_deposit_recipe(collateral_token: Address) -> RecipeDefinition {
        let mut recipe = borrow_recipe(U256::from(1_000_000_000_000_000_000_u128));
        recipe.nodes.push(RecipeNode {
            id: EARN_ID,
            primitive: RecipePrimitive::EarnDeposit {
                collateral_token,
                investment_id: INVESTMENT_ID.to_owned(),
            },
        });
        recipe.edges.push(RecipeEdge {
            id: Uuid::from_u128(15),
            from: RecipePortRef {
                node_id: BORROW_ID,
                port: "borrowed".to_owned(),
            },
            to: RecipePortRef {
                node_id: EARN_ID,
                port: "asset".to_owned(),
            },
        });
        recipe
    }

    fn repayment_recipe(pool: Address, token: Address) -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: DEBT_ID,
                    primitive: RecipePrimitive::BorrowedPosition {
                        venue: BorrowingVenue::Venus,
                        pool,
                        token,
                    },
                },
                RecipeNode {
                    id: REPAY_ID,
                    primitive: RecipePrimitive::Repay,
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(13),
                from: RecipePortRef {
                    node_id: DEBT_ID,
                    port: "debt".to_owned(),
                },
                to: RecipePortRef {
                    node_id: REPAY_ID,
                    port: "debt".to_owned(),
                },
            }],
        }
    }

    fn withdrawal_account(has_position: bool, total_borrowed_usd: Decimal) -> BorrowingAccount {
        let supplied = has_position
            .then_some(BorrowingAsset {
                token: BorrowingToken {
                    contract: BSTOCK,
                    symbol: "TSLAB".to_owned(),
                },
                amount: Decimal::new(193, 4),
                value_usd: Decimal::new(734, 2),
            })
            .into_iter()
            .collect::<Vec<_>>();
        BorrowingAccount {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            positions: vec![BorrowingPosition {
                pool: POOL,
                supplied,
                borrowed: Vec::new(),
            }],
            total_supplied_usd: if has_position {
                Decimal::new(734, 2)
            } else {
                Decimal::ZERO
            },
            total_borrowed_usd,
            health_factor: None,
            observed_at_ms: 1,
        }
    }

    fn states(preview: &RecipePreview) -> Vec<RecipeNodeState> {
        preview
            .nodes
            .iter()
            .map(|node| match node {
                RecipeNodePreview::Trade { state, .. }
                | RecipeNodePreview::Supply { state, .. }
                | RecipeNodePreview::Withdraw { state, .. }
                | RecipeNodePreview::Borrow { state, .. }
                | RecipeNodePreview::Repay { state, .. }
                | RecipeNodePreview::EarnDeposit { state, .. }
                | RecipeNodePreview::EarnWithdraw { state, .. } => *state,
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

    fn borrowing_transaction(kind: BorrowingTransactionKind) -> PreparedBorrowingTransaction {
        PreparedBorrowingTransaction {
            kind,
            to: Address::with_last_byte(7),
            data: Bytes::new(),
            value: U256::ZERO,
            gas_limit: None,
            gas_price: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
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
