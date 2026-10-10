use std::{collections::HashMap, sync::Arc};

use alloy_primitives::Address;
use thiserror::Error;

use super::{
    ComposeContext, ComposeStrategyRequest, ComposedStrategy, RejectedDraft, StrategyDraft,
    StrategyModel, StrategyModelError,
    resolve::{Resolution, Resolver, step_index},
};
use crate::{
    bstock_borrowing::{BorrowingService, BorrowingServiceError},
    bstock_trade::SettlementTokenMetadata,
    carry::{CarryOpportunityQuery, CarryQualification, CarryService},
    recipes::{
        RecipeDefinition, RecipeDiagnostic, RecipeDiagnosticLocation, RecipeDiagnosticSeverity,
        RecipeError, RecipePrimitive, RecipeService,
    },
};

const MAX_PROMPT_CHARS: usize = 1_000;
/// One draft plus at most two repairs keeps latency and free-tier quota bounded.
const MAX_MODEL_CALLS: usize = 3;

#[derive(Debug, Error)]
pub enum StrategyComposerError {
    #[error("the strategy prompt must contain between 1 and {MAX_PROMPT_CHARS} characters")]
    InvalidPrompt,
    #[error(transparent)]
    Recipe(#[from] RecipeError),
    #[error(transparent)]
    Borrowing(#[from] BorrowingServiceError),
    #[error(transparent)]
    Model(#[from] StrategyModelError),
}

pub struct StrategyComposerService {
    recipes: Arc<RecipeService>,
    borrowing: Arc<BorrowingService>,
    carry: Arc<CarryService>,
    model: Arc<dyn StrategyModel>,
    settlement_tokens: Vec<SettlementTokenMetadata>,
}

/// What filling in the earn opportunities told the user.
struct EarnSelection {
    assumptions: Vec<String>,
    missing: Vec<String>,
}

impl StrategyComposerService {
    #[must_use]
    pub fn new(
        recipes: Arc<RecipeService>,
        borrowing: Arc<BorrowingService>,
        carry: Arc<CarryService>,
        model: Arc<dyn StrategyModel>,
        settlement_tokens: Vec<SettlementTokenMetadata>,
    ) -> Self {
        Self {
            recipes,
            borrowing,
            carry,
            model,
            settlement_tokens,
        }
    }

    /// Chooses the reviewed earn product behind every earn deposit.
    ///
    /// Which products are depositable is live policy the model cannot know, so
    /// the chosen route is reported as an assumption rather than left implicit.
    async fn select_earn_opportunities(
        &self,
        recipe: &mut RecipeDefinition,
    ) -> Result<EarnSelection, StrategyComposerError> {
        let mut selection = EarnSelection {
            assumptions: Vec::new(),
            missing: Vec::new(),
        };
        let sources = recipe
            .edges
            .iter()
            .map(|edge| (edge.to.node_id, edge.from.node_id))
            .collect::<HashMap<_, _>>();
        let deposited = recipe
            .nodes
            .iter()
            .map(|node| (node.id, deposit_token(&node.primitive)))
            .collect::<HashMap<_, _>>();
        for node in &mut recipe.nodes {
            let RecipePrimitive::EarnDeposit {
                collateral_token,
                investment_id,
            } = &mut node.primitive
            else {
                continue;
            };
            if !investment_id.is_empty() {
                continue;
            }
            let Some(debt_token) = sources
                .get(&node.id)
                .and_then(|source| deposited.get(source).copied().flatten())
            else {
                selection
                    .missing
                    .push("Which stablecoin should the earn deposit use?".to_owned());
                continue;
            };
            // The draft is worth keeping when the catalog is briefly unreadable:
            // the recipe could not be signed either way, so ask rather than fail.
            let Ok(opportunities) = self
                .carry
                .opportunities(CarryOpportunityQuery {
                    collateral_token: *collateral_token,
                    debt_token,
                })
                .await
            else {
                selection.missing.push(
                    "Earn opportunities are temporarily unavailable. Pick one in the builder."
                        .to_owned(),
                );
                continue;
            };
            let best = opportunities
                .into_iter()
                .filter(|opportunity| opportunity.qualification == CarryQualification::Eligible)
                .max_by_key(|opportunity| opportunity.base_earn_rate_bps);
            match best {
                Some(opportunity) => {
                    selection.assumptions.push(format!(
                        "Deposits into {} on {}, earning {} against a {}% borrow rate.",
                        opportunity.investment_name,
                        opportunity.protocol_name,
                        opportunity.base_earn_rate_display,
                        opportunity.borrow_apy_percent.round_dp(2)
                    ));
                    *investment_id = opportunity.investment_id;
                }
                None => selection
                    .missing
                    .push("No reviewed earn product currently clears the borrow rate.".to_owned()),
            }
        }
        Ok(selection)
    }

    /// Drafts a strategy with the model, then resolves and validates it as a recipe.
    ///
    /// Model mistakes are fed back for repair. Questions only the user can answer,
    /// such as a missing amount, end composition without a recipe.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyComposerError`] for an invalid prompt, unavailable live data,
    /// or a model that is unavailable or never returns a usable draft.
    pub async fn compose(
        &self,
        request: &ComposeStrategyRequest,
    ) -> Result<ComposedStrategy, StrategyComposerError> {
        let prompt = request.prompt.trim();
        if prompt.is_empty() || prompt.chars().count() > MAX_PROMPT_CHARS {
            return Err(StrategyComposerError::InvalidPrompt);
        }
        let catalog = self.recipes.catalog().await?;
        let account = match request.wallet {
            Some(wallet) => Some(self.borrowing.account(wallet).await?),
            None => None,
        };
        let resolver = Resolver::new(
            &catalog.bstocks,
            &self.settlement_tokens,
            &catalog.borrow_policy.debt_tokens,
            catalog
                .borrow_policy
                .allowed_inputs
                .first()
                .map(|rule| rule.pool),
            account.as_ref(),
        );
        let mut context = ComposeContext {
            prompt: prompt.to_owned(),
            assets: resolver.assets(),
            positions: resolver.positions(),
            previous: request.previous.clone(),
            rejected: None,
        };

        let mut calls = 0;
        loop {
            calls += 1;
            let final_call = calls == MAX_MODEL_CALLS;
            let mut draft = match self.model.draft(&context).await {
                Ok(draft) => draft,
                Err(StrategyModelError::InvalidResponse) if !final_call => continue,
                Err(error) => return Err(error.into()),
            };
            if draft.intent.steps.is_empty() {
                return Ok(without_recipe(draft, Vec::new()));
            }
            let issues = match resolver.resolve(&draft.intent) {
                Resolution::NeedsInput(questions) => return Ok(without_recipe(draft, questions)),
                Resolution::Rejected(issues) => issues,
                Resolution::Recipe(mut recipe) => {
                    let selection = self.select_earn_opportunities(&mut recipe).await?;
                    if !selection.missing.is_empty() {
                        return Ok(without_recipe(draft, selection.missing));
                    }
                    draft.assumptions.extend(selection.assumptions);
                    let validation = self.recipes.validate(&recipe);
                    if validation.valid {
                        return Ok(ComposedStrategy {
                            summary: draft.summary,
                            intent: draft.intent,
                            recipe: Some(recipe),
                            assumptions: draft.assumptions,
                            missing_inputs: Vec::new(),
                            validation: Some(validation),
                        });
                    }
                    if final_call {
                        let mut composed = without_recipe(draft, Vec::new());
                        composed.validation = Some(validation);
                        return Ok(composed);
                    }
                    validation
                        .diagnostics
                        .iter()
                        .filter(|diagnostic| diagnostic.severity == RecipeDiagnosticSeverity::Error)
                        .map(describe)
                        .collect()
                }
            };
            if final_call {
                return Ok(without_recipe(draft, issues));
            }
            context.rejected = Some(RejectedDraft {
                intent: draft.intent,
                issues,
            });
        }
    }
}

/// The token a node leaves for the next step, when it leaves one.
const fn deposit_token(primitive: &RecipePrimitive) -> Option<Address> {
    match primitive {
        RecipePrimitive::WalletAsset { token, .. } => Some(*token),
        RecipePrimitive::Borrow { debt_token, .. } => Some(*debt_token),
        RecipePrimitive::Trade { buy_token } => Some(*buy_token),
        _ => None,
    }
}

fn without_recipe(draft: StrategyDraft, missing_inputs: Vec<String>) -> ComposedStrategy {
    ComposedStrategy {
        summary: draft.summary,
        intent: draft.intent,
        recipe: None,
        assumptions: draft.assumptions,
        missing_inputs,
        validation: None,
    }
}

fn describe(diagnostic: &RecipeDiagnostic) -> String {
    let node = match &diagnostic.location {
        RecipeDiagnosticLocation::Node { node_id }
        | RecipeDiagnosticLocation::Field { node_id, .. }
        | RecipeDiagnosticLocation::Port { node_id, .. } => Some(*node_id),
        RecipeDiagnosticLocation::Recipe | RecipeDiagnosticLocation::Edge { .. } => None,
    };
    match node.and_then(step_index) {
        Some(index) => format!("Step {}: {}", index + 1, diagnostic.message),
        None => diagnostic.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use alloy_primitives::{Address, B256};
    use async_trait::async_trait;
    use rust_decimal::Decimal;

    use super::*;
    use crate::{
        bstock_borrowing::{
            BorrowCapability, BorrowCapabilityRequest, BorrowingAccount, BorrowingActionGateway,
            BorrowingMarket, BorrowingPolicy, BorrowingProviderError, BorrowingReader,
            BorrowingToken, BorrowingTransactionStatus, BorrowingVenue, PrepareBorrowRequest,
            PrepareCollateralRequest, PrepareRepaymentRequest, PrepareSupplyRequest,
            PrepareWithdrawalRequest, PreparedBorrowingAction, RepaymentCapability,
            RepaymentCapabilityRequest, SupplyMarket,
        },
        bstock_market::{
            AssetKind, BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockMarketService,
            BstockSummary, CatalogError,
        },
        bstock_trade::{
            ApprovalRequest, BstockTradeService, PrepareTradeRequest, PreparedApproval,
            PreparedTrade, SubmitOrderRequest, SubmittedOrder, TradeGateway, TradeGatewayError,
            TradeOrderStatus, TradePolicy, TradeQuote, TradeQuoteRequest, TradeTransactionStatus,
        },
        carry::{
            CarryAccount, CarryActionPlan, CarryPolicy, CarryService, EarnActionGateway,
            EarnDepositCommand, EarnOpportunity, EarnProviderError, EarnRateType, EarnReader,
            EarnRedemptionClaimCommand, EarnWithdrawalCommand,
        },
        strategy_composer::{StrategyIntent, StrategyStep},
        wallet_portfolio::WalletAddress,
    };

    const USDT: Address = Address::with_last_byte(1);
    const TSLAB: Address = Address::with_last_byte(2);
    const POOL: Address = Address::with_last_byte(9);
    const BEST_INVESTMENT: &str = "best";

    #[tokio::test]
    async fn invalid_drafts_are_repaired_with_feedback_until_valid() {
        let model = Arc::new(ScriptedModel::new(vec![
            Ok(draft(vec![StrategyStep::Supply])),
            Ok(draft(vec![
                wallet_usdt("6"),
                StrategyStep::Trade {
                    buy_symbol: "TSLAB".to_owned(),
                },
            ])),
        ]));

        let composed = composer(model.clone())
            .compose(&request("buy tesla with 6 usdt"))
            .await
            .unwrap_or_else(|error| panic!("composition failed: {error}"));

        assert!(composed.recipe.is_some());
        assert_eq!(composed.validation.map(|report| report.valid), Some(true));
        let contexts = model.contexts();
        assert_eq!(contexts.len(), 2);
        assert!(contexts[0].rejected.is_none());
        assert!(contexts[1].rejected.is_some());
    }

    #[tokio::test]
    async fn an_earn_deposit_takes_the_best_reviewed_product_and_says_which() {
        let model = Arc::new(ScriptedModel::new(vec![Ok(draft(vec![
            wallet_usdt("10"),
            StrategyStep::EarnDeposit {
                symbol: "TSLAB".to_owned(),
            },
        ]))]));

        let composed = composer(model)
            .compose(&request("earn on 10 usdt against my tesla"))
            .await
            .unwrap_or_else(|error| panic!("composition failed: {error}"));

        let recipe = composed.recipe.unwrap_or_else(|| {
            panic!(
                "earn deposit did not resolve: {:?}",
                composed.missing_inputs
            )
        });
        let investment = recipe.nodes.iter().find_map(|node| match &node.primitive {
            RecipePrimitive::EarnDeposit { investment_id, .. } => Some(investment_id.clone()),
            _ => None,
        });
        assert_eq!(investment.as_deref(), Some(BEST_INVESTMENT));
        assert!(
            composed
                .assumptions
                .iter()
                .any(|assumption| assumption.contains("best vault")),
            "the chosen product is not stated: {:?}",
            composed.assumptions
        );
    }

    #[tokio::test]
    async fn persistent_invalid_drafts_stop_after_the_call_budget_without_a_recipe() {
        let model = Arc::new(ScriptedModel::new(vec![
            Ok(draft(vec![StrategyStep::Supply])),
            Ok(draft(vec![StrategyStep::Supply])),
            Ok(draft(vec![StrategyStep::Supply])),
            Ok(draft(vec![StrategyStep::Supply])),
        ]));

        let composed = composer(model.clone())
            .compose(&request("supply"))
            .await
            .unwrap_or_else(|error| panic!("composition failed: {error}"));

        assert!(composed.recipe.is_none());
        assert!(!composed.missing_inputs.is_empty());
        assert_eq!(model.contexts().len(), MAX_MODEL_CALLS);
    }

    #[tokio::test]
    async fn questions_for_the_user_end_composition_without_another_model_call() {
        let model = Arc::new(ScriptedModel::new(vec![Ok(draft(vec![
            StrategyStep::WalletAsset {
                symbol: "USDT".to_owned(),
                amount: None,
            },
        ]))]));

        let composed = composer(model.clone())
            .compose(&request("buy some tesla"))
            .await
            .unwrap_or_else(|error| panic!("composition failed: {error}"));

        assert!(composed.recipe.is_none());
        assert_eq!(composed.missing_inputs.len(), 1);
        assert_eq!(model.contexts().len(), 1);
    }

    #[tokio::test]
    async fn rate_limits_surface_immediately_and_blank_prompts_never_call_the_model() {
        let model = Arc::new(ScriptedModel::new(vec![Err(
            StrategyModelError::RateLimited,
        )]));
        let service = composer(model.clone());

        let blank = service.compose(&request("   ")).await;
        let limited = service.compose(&request("buy tesla")).await;

        assert!(matches!(blank, Err(StrategyComposerError::InvalidPrompt)));
        assert!(matches!(
            limited,
            Err(StrategyComposerError::Model(
                StrategyModelError::RateLimited
            ))
        ));
        assert_eq!(model.contexts().len(), 1);
    }

    struct ScriptedModel {
        responses: Mutex<Vec<Result<StrategyDraft, StrategyModelError>>>,
        contexts: Mutex<Vec<ComposeContext>>,
    }

    impl ScriptedModel {
        fn new(mut responses: Vec<Result<StrategyDraft, StrategyModelError>>) -> Self {
            responses.reverse();
            Self {
                responses: Mutex::new(responses),
                contexts: Mutex::new(Vec::new()),
            }
        }

        fn contexts(&self) -> Vec<ComposeContext> {
            self.contexts
                .lock()
                .unwrap_or_else(|error| panic!("contexts lock poisoned: {error}"))
                .clone()
        }
    }

    #[async_trait]
    impl StrategyModel for ScriptedModel {
        async fn draft(
            &self,
            context: &ComposeContext,
        ) -> Result<StrategyDraft, StrategyModelError> {
            self.contexts
                .lock()
                .unwrap_or_else(|error| panic!("contexts lock poisoned: {error}"))
                .push(context.clone());
            self.responses
                .lock()
                .unwrap_or_else(|error| panic!("responses lock poisoned: {error}"))
                .pop()
                .unwrap_or(Err(StrategyModelError::Unavailable))
        }
    }

    fn composer(model: Arc<ScriptedModel>) -> StrategyComposerService {
        let catalog = Arc::new(FixedCatalog);
        let market = Arc::new(BstockMarketService::new(catalog.clone()));
        let policy = TradePolicy::new([USDT], 100, 50)
            .unwrap_or_else(|error| panic!("valid trade policy rejected: {error}"));
        let trade = Arc::new(BstockTradeService::new(
            Arc::new(Unused),
            catalog.clone(),
            policy,
        ));
        let borrowing = Arc::new(BorrowingService::new(
            catalog,
            Arc::new(Unused),
            Arc::new(Unused),
            BorrowingPolicy::new(vec![USDT])
                .unwrap_or_else(|error| panic!("valid borrowing policy rejected: {error}")),
        ));
        let carry = Arc::new(CarryService::new(
            borrowing.clone(),
            Arc::new(Unused),
            Arc::new(Unused),
            CarryPolicy::new(vec![BEST_INVESTMENT.to_owned(), "lesser".to_owned()], 100),
        ));
        let recipes = Arc::new(RecipeService::new(
            market,
            trade,
            borrowing.clone(),
            carry.clone(),
            vec![SettlementTokenMetadata {
                address: USDT,
                symbol: "USDT".to_owned(),
                decimals: 18,
            }],
            POOL,
        ));
        StrategyComposerService::new(
            recipes,
            borrowing,
            carry,
            model,
            vec![SettlementTokenMetadata {
                address: USDT,
                symbol: "USDT".to_owned(),
                decimals: 18,
            }],
        )
    }

    fn request(prompt: &str) -> ComposeStrategyRequest {
        ComposeStrategyRequest {
            prompt: prompt.to_owned(),
            wallet: None,
            previous: None,
        }
    }

    fn draft(steps: Vec<StrategyStep>) -> StrategyDraft {
        StrategyDraft {
            summary: "test strategy".to_owned(),
            intent: StrategyIntent { steps },
            assumptions: Vec::new(),
        }
    }

    fn wallet_usdt(amount: &str) -> StrategyStep {
        StrategyStep::WalletAsset {
            symbol: "USDT".to_owned(),
            amount: Some(
                amount
                    .parse::<Decimal>()
                    .unwrap_or_else(|error| panic!("test amount is invalid: {error}")),
            ),
        }
    }

    fn earn(investment_id: &str, base_rate_bps: i64) -> EarnOpportunity {
        EarnOpportunity {
            investment_id: investment_id.to_owned(),
            investment_name: format!("{investment_id} vault"),
            protocol_id: "lista".to_owned(),
            protocol_name: "Lista".to_owned(),
            protocol_logo_url: None,
            asset: BorrowingToken {
                contract: USDT,
                symbol: "USDT".to_owned(),
            },
            rate_type: EarnRateType::Apy,
            base_rate_bps,
            base_rate_display: format!("{}.00%", base_rate_bps / 100),
            reported_tvl_usd: None,
            investable: true,
            observed_at_ms: 1,
        }
    }

    struct FixedCatalog;

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(vec![BstockSummary {
                id: BstockId {
                    chain_id: BNB_CHAIN_ID,
                    contract: TSLAB,
                },
                token_name: "Tesla (bStocks)".to_owned(),
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
            }])
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    /// Composition must never quote, prepare or read capabilities.
    struct Unused;

    #[async_trait]
    impl TradeGateway for Unused {
        async fn quotes(
            &self,
            _: &TradeQuoteRequest,
        ) -> Result<Vec<TradeQuote>, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
        async fn prepare_approval(
            &self,
            _: &ApprovalRequest,
        ) -> Result<PreparedApproval, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
        async fn prepare_trade(
            &self,
            _: &PrepareTradeRequest,
            _: Decimal,
        ) -> Result<PreparedTrade, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
        async fn submit_order(
            &self,
            _: &SubmitOrderRequest,
        ) -> Result<SubmittedOrder, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
        async fn order_status(&self, _: &str) -> Result<TradeOrderStatus, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
        async fn transaction_status(
            &self,
            _: B256,
        ) -> Result<TradeTransactionStatus, TradeGatewayError> {
            Err(TradeGatewayError::Unavailable)
        }
    }

    #[async_trait]
    impl EarnReader for Unused {
        async fn opportunities(
            &self,
            _asset: Address,
        ) -> Result<Vec<EarnOpportunity>, EarnProviderError> {
            Ok(vec![earn("lesser", 600), earn(BEST_INVESTMENT, 900)])
        }

        async fn opportunity(
            &self,
            _investment_id: &str,
        ) -> Result<Option<EarnOpportunity>, EarnProviderError> {
            unreachable!("the composer never reads an earn opportunity")
        }

        async fn positions(
            &self,
            _wallet: WalletAddress,
        ) -> Result<CarryAccount, EarnProviderError> {
            unreachable!("the composer never reads earn positions")
        }
    }

    #[async_trait]
    impl EarnActionGateway for Unused {
        async fn prepare_deposit(
            &self,
            _command: &EarnDepositCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            unreachable!("the composer never prepares an earn deposit")
        }

        async fn prepare_withdrawal(
            &self,
            _command: &EarnWithdrawalCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            unreachable!("the composer never prepares an earn withdrawal")
        }

        async fn prepare_redemption_claim(
            &self,
            _command: &EarnRedemptionClaimCommand,
        ) -> Result<CarryActionPlan, EarnProviderError> {
            unreachable!("the composer never claims an earn redemption")
        }
    }

    #[async_trait]
    impl BorrowingReader for Unused {
        async fn read_borrow_capability(
            &self,
            _: &BorrowCapabilityRequest,
        ) -> Result<BorrowCapability, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn read_repayment_capability(
            &self,
            _: &RepaymentCapabilityRequest,
        ) -> Result<RepaymentCapability, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn read_supply_market(
            &self,
            _: Address,
        ) -> Result<Option<SupplyMarket>, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn read_markets(
            &self,
            _: &[BstockId],
            _: &[Address],
        ) -> Result<Vec<BorrowingMarket>, BorrowingProviderError> {
            Ok(vec![BorrowingMarket {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                collateral: BorrowingToken {
                    contract: TSLAB,
                    symbol: "TSLAB".to_owned(),
                },
                collateral_market: POOL,
                debt: BorrowingToken {
                    contract: USDT,
                    symbol: "USDT".to_owned(),
                },
                debt_market: POOL,
                collateral_factor: Decimal::new(6, 1),
                liquidation_threshold: Decimal::new(7, 1),
                remaining_supply_capacity: Decimal::from(1_000),
                available_borrow_liquidity: Decimal::from(1_000),
                borrow_apy_percent: Decimal::from(5),
                supply_available: true,
                borrow_available: true,
                oracle_valid: true,
                unavailable_reasons: Vec::new(),
                observed_at_ms: 1,
            }])
        }
        async fn read_account(
            &self,
            _: WalletAddress,
        ) -> Result<BorrowingAccount, BorrowingProviderError> {
            Ok(BorrowingAccount {
                venue: BorrowingVenue::Venus,
                chain_id: BNB_CHAIN_ID,
                positions: Vec::new(),
                total_supplied_usd: Decimal::ZERO,
                total_borrowed_usd: Decimal::ZERO,
                health_factor: None,
                observed_at_ms: 0,
            })
        }
    }

    #[async_trait]
    impl BorrowingActionGateway for Unused {
        async fn prepare_supply(
            &self,
            _: &PrepareSupplyRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_collateral(
            &self,
            _: &PrepareCollateralRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_borrow(
            &self,
            _: &PrepareBorrowRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_repayment_approval(
            &self,
            _: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_repayment(
            &self,
            _: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_full_repayment(
            &self,
            _: &PrepareRepaymentRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn prepare_withdrawal(
            &self,
            _: &PrepareWithdrawalRequest,
        ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
        async fn transaction_status(
            &self,
            _: B256,
        ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
            Err(BorrowingProviderError::Unavailable)
        }
    }
}
