//! Pure recipe validation and lowering.
//!
//! The compiler accepts JSON-shaped recipe data and produces a logical plan.
//! It performs no network calls and reads no database state, so the same input
//! always produces the same report and plan.
//!
//! The flow is deliberately small:
//!
//! ```text
//! RecipeDefinition
//!   -> validate fields and graph
//!   -> order nodes by dependency
//!   -> lower nodes into provider-independent actions
//!   -> LogicalPlan
//! ```

use std::collections::HashMap;

use uuid::Uuid;

use super::{
    AssetRef, BorrowedPositionRef, LogicalPlan, LogicalStep, RecipeAction, RecipeDefinition,
    RecipeDiagnostic, RecipeDiagnosticCode, RecipeDiagnosticLocation, RecipeGraph, RecipePrimitive,
    RecipeValidationReport, SuppliedPositionRef,
};
use crate::bstock_market::BNB_CHAIN_ID;

pub const SCHEMA_VERSION: u16 = 1;
const MAX_NODES: usize = 32;
const MAX_EDGES: usize = 64;

pub(super) struct RecipeCompilation {
    pub report: RecipeValidationReport,
    pub plan: Option<LogicalPlan>,
}

pub(super) fn compile(recipe: &RecipeDefinition) -> RecipeCompilation {
    let mut diagnostics = validate_recipe_header(recipe);
    validate_node_fields(recipe, &mut diagnostics);
    let graph = RecipeGraph::check(recipe, &mut diagnostics);

    let mut report = RecipeValidationReport::new(diagnostics);
    let plan = if report.valid {
        match lower(&graph) {
            Ok(plan) => Some(plan),
            Err(diagnostic) => {
                report.diagnostics.push(diagnostic);
                report.valid = false;
                None
            }
        }
    } else {
        None
    };
    RecipeCompilation { report, plan }
}

fn validate_recipe_header(recipe: &RecipeDefinition) -> Vec<RecipeDiagnostic> {
    let mut diagnostics = Vec::new();
    if recipe.schema_version != SCHEMA_VERSION {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedSchemaVersion,
            RecipeDiagnosticLocation::Recipe,
            format!("schemaVersion must be {SCHEMA_VERSION}"),
        ));
    }
    if recipe.chain_id != BNB_CHAIN_ID {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::UnsupportedChain,
            RecipeDiagnosticLocation::Recipe,
            "recipes are currently supported only on BNB Chain (56)",
        ));
    }
    if recipe.nodes.is_empty() {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::EmptyRecipe,
            RecipeDiagnosticLocation::Recipe,
            "add at least one node to the recipe",
        ));
    }
    if recipe.nodes.len() > MAX_NODES || recipe.edges.len() > MAX_EDGES {
        diagnostics.push(RecipeDiagnostic::error(
            RecipeDiagnosticCode::RecipeTooLarge,
            RecipeDiagnosticLocation::Recipe,
            format!("a recipe may contain at most {MAX_NODES} nodes and {MAX_EDGES} edges"),
        ));
    }
    diagnostics
}

fn validate_node_fields(recipe: &RecipeDefinition, diagnostics: &mut Vec<RecipeDiagnostic>) {
    for node in &recipe.nodes {
        if matches!(
            &node.primitive,
            RecipePrimitive::WalletAsset { raw_amount, .. }
                | RecipePrimitive::Borrow { raw_amount, .. }
                if raw_amount.is_zero()
        ) {
            diagnostics.push(RecipeDiagnostic::error(
                RecipeDiagnosticCode::InvalidAmount,
                RecipeDiagnosticLocation::Field {
                    node_id: node.id,
                    field: "rawAmount".to_owned(),
                },
                "amount must be greater than zero",
            ));
        }
    }
}

fn lower(graph: &RecipeGraph<'_>) -> Result<LogicalPlan, RecipeDiagnostic> {
    let mut values = HashMap::<(Uuid, String), AssetRef>::new();
    let mut positions = HashMap::<(Uuid, String), SuppliedPositionRef>::new();
    let mut debts = HashMap::<(Uuid, String), BorrowedPositionRef>::new();
    let mut steps = Vec::new();

    for node_id in graph.topological_order() {
        let Some(node) = graph.node(node_id) else {
            return Err(missing_input(node_id, "node"));
        };
        match &node.primitive {
            RecipePrimitive::WalletAsset { token, raw_amount } => {
                values.insert(
                    (node_id, "asset".to_owned()),
                    AssetRef::Wallet {
                        token: *token,
                        amount: *raw_amount,
                    },
                );
            }
            RecipePrimitive::SuppliedPosition { venue, pool, token } => {
                register_supplied_position(&mut positions, node_id, *venue, *pool, *token);
            }
            RecipePrimitive::BorrowedPosition { venue, pool, token } => {
                register_borrowed_position(&mut debts, node_id, *venue, *pool, *token);
            }
            RecipePrimitive::Trade { buy_token } => {
                let (sell, dependency) = input_asset(
                    graph,
                    &values,
                    node_id,
                    "sell",
                    "the trade input is not available before this step",
                )?;
                steps.push(LogicalStep {
                    node_id,
                    dependencies: matches!(sell, AssetRef::NodeOutput { .. })
                        .then_some(dependency)
                        .into_iter()
                        .collect(),
                    action: RecipeAction::Trade {
                        sell,
                        buy_token: *buy_token,
                    },
                });
                values.insert(
                    (node_id, "buy".to_owned()),
                    AssetRef::NodeOutput {
                        node_id,
                        port: "buy".to_owned(),
                        token: *buy_token,
                    },
                );
            }
            RecipePrimitive::Supply { venue } => {
                let (asset, dependency) = input_asset(
                    graph,
                    &values,
                    node_id,
                    "asset",
                    "the supply input is not available before this step",
                )?;
                steps.push(LogicalStep {
                    node_id,
                    dependencies: matches!(asset, AssetRef::NodeOutput { .. })
                        .then_some(dependency)
                        .into_iter()
                        .collect(),
                    action: RecipeAction::Supply {
                        asset,
                        venue: *venue,
                    },
                });
            }
            RecipePrimitive::Withdraw => {
                let position = input_position(graph, &positions, node_id)?;
                steps.push(LogicalStep {
                    node_id,
                    dependencies: Vec::new(),
                    action: RecipeAction::Withdraw { position },
                });
            }
            RecipePrimitive::Borrow {
                debt_token,
                raw_amount,
            } => {
                let (step, output) =
                    lower_borrow(graph, &positions, node_id, *debt_token, *raw_amount)?;
                steps.push(step);
                values.insert((node_id, "borrowed".to_owned()), output);
            }
            RecipePrimitive::Repay => {
                let debt = input_debt(graph, &debts, node_id)?;
                steps.push(LogicalStep {
                    node_id,
                    dependencies: Vec::new(),
                    action: RecipeAction::Repay { debt },
                });
            }
        }
    }
    Ok(LogicalPlan { steps })
}

fn register_supplied_position(
    positions: &mut HashMap<(Uuid, String), SuppliedPositionRef>,
    node_id: Uuid,
    venue: crate::bstock_borrowing::BorrowingVenue,
    pool: alloy_primitives::Address,
    token: alloy_primitives::Address,
) {
    positions.insert(
        (node_id, "position".to_owned()),
        SuppliedPositionRef { venue, pool, token },
    );
}

fn register_borrowed_position(
    debts: &mut HashMap<(Uuid, String), BorrowedPositionRef>,
    node_id: Uuid,
    venue: crate::bstock_borrowing::BorrowingVenue,
    pool: alloy_primitives::Address,
    token: alloy_primitives::Address,
) {
    debts.insert(
        (node_id, "debt".to_owned()),
        BorrowedPositionRef { venue, pool, token },
    );
}

fn lower_borrow(
    graph: &RecipeGraph<'_>,
    positions: &HashMap<(Uuid, String), SuppliedPositionRef>,
    node_id: Uuid,
    debt_token: alloy_primitives::Address,
    raw_amount: alloy_primitives::U256,
) -> Result<(LogicalStep, AssetRef), RecipeDiagnostic> {
    let collateral = input_position_on(graph, positions, node_id, "collateral")?;
    Ok((
        LogicalStep {
            node_id,
            dependencies: Vec::new(),
            action: RecipeAction::Borrow {
                collateral,
                debt_token,
                raw_amount,
            },
        },
        AssetRef::NodeOutput {
            node_id,
            port: "borrowed".to_owned(),
            token: debt_token,
        },
    ))
}

fn input_asset(
    graph: &RecipeGraph<'_>,
    values: &HashMap<(Uuid, String), AssetRef>,
    node_id: Uuid,
    port: &str,
    unavailable_message: &'static str,
) -> Result<(AssetRef, Uuid), RecipeDiagnostic> {
    let edge = graph
        .incoming(node_id, port)
        .ok_or_else(|| missing_input(node_id, port))?;
    let source_id = edge.edge.from.node_id;
    let source_key = (source_id, edge.edge.from.port.clone());
    let asset = values
        .get(&source_key)
        .cloned()
        .ok_or_else(|| unavailable_input(node_id, port, unavailable_message))?;
    Ok((asset, source_id))
}

fn input_position(
    graph: &RecipeGraph<'_>,
    positions: &HashMap<(Uuid, String), SuppliedPositionRef>,
    node_id: Uuid,
) -> Result<SuppliedPositionRef, RecipeDiagnostic> {
    input_position_on(graph, positions, node_id, "position")
}

fn input_position_on(
    graph: &RecipeGraph<'_>,
    positions: &HashMap<(Uuid, String), SuppliedPositionRef>,
    node_id: Uuid,
    port: &str,
) -> Result<SuppliedPositionRef, RecipeDiagnostic> {
    let edge = graph
        .incoming(node_id, port)
        .ok_or_else(|| missing_input(node_id, port))?;
    let source_key = (edge.edge.from.node_id, edge.edge.from.port.clone());
    positions.get(&source_key).copied().ok_or_else(|| {
        unavailable_input(
            node_id,
            port,
            "the supplied position is not available before this step",
        )
    })
}

fn input_debt(
    graph: &RecipeGraph<'_>,
    debts: &HashMap<(Uuid, String), BorrowedPositionRef>,
    node_id: Uuid,
) -> Result<BorrowedPositionRef, RecipeDiagnostic> {
    let edge = graph
        .incoming(node_id, "debt")
        .ok_or_else(|| missing_input(node_id, "debt"))?;
    let source_key = (edge.edge.from.node_id, edge.edge.from.port.clone());
    debts.get(&source_key).copied().ok_or_else(|| {
        unavailable_input(
            node_id,
            "debt",
            "the borrowed position is not available before this step",
        )
    })
}

fn unavailable_input(node_id: Uuid, port: &str, message: &'static str) -> RecipeDiagnostic {
    RecipeDiagnostic::error(
        RecipeDiagnosticCode::MissingRequiredInput,
        RecipeDiagnosticLocation::Port {
            node_id,
            port: port.to_owned(),
        },
        message,
    )
}

fn missing_input(node_id: Uuid, port: &str) -> RecipeDiagnostic {
    RecipeDiagnostic::error(
        RecipeDiagnosticCode::MissingRequiredInput,
        RecipeDiagnosticLocation::Port {
            node_id,
            port: port.to_owned(),
        },
        "a required input is missing",
    )
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};

    use super::*;
    use crate::bstock_borrowing::BorrowingVenue;
    use crate::recipes::{RecipeEdge, RecipeNode, RecipePortRef};

    const SOURCE_ID: Uuid = Uuid::from_u128(1);
    const TRADE_ID: Uuid = Uuid::from_u128(2);
    const SUPPLY_ID: Uuid = Uuid::from_u128(6);
    const POSITION_ID: Uuid = Uuid::from_u128(8);
    const WITHDRAW_ID: Uuid = Uuid::from_u128(9);
    const BORROW_ID: Uuid = Uuid::from_u128(12);
    const DEBT_ID: Uuid = Uuid::from_u128(13);
    const REPAY_ID: Uuid = Uuid::from_u128(14);

    #[test]
    fn compiles_wallet_asset_trade_into_a_logical_plan() {
        let compiled = compile(&recipe());

        assert!(compiled.report.valid);
        let plan = compiled
            .plan
            .unwrap_or_else(|| panic!("valid recipe did not produce a plan"));
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].node_id, TRADE_ID);
        assert_eq!(
            plan.steps[0].action,
            RecipeAction::Trade {
                sell: AssetRef::Wallet {
                    token: Address::with_last_byte(1),
                    amount: U256::from(6),
                },
                buy_token: Address::with_last_byte(2),
            }
        );
    }

    #[test]
    fn reports_all_independent_editor_errors_at_once() {
        let mut recipe = recipe();
        recipe.chain_id = 1;
        recipe.edges.clear();
        if let RecipePrimitive::WalletAsset { raw_amount, .. } = &mut recipe.nodes[0].primitive {
            *raw_amount = U256::ZERO;
        }

        let compiled = compile(&recipe);
        let codes = compiled
            .report
            .diagnostics
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>();

        assert!(!compiled.report.valid);
        assert!(compiled.plan.is_none());
        assert!(codes.contains(&RecipeDiagnosticCode::UnsupportedChain));
        assert!(codes.contains(&RecipeDiagnosticCode::InvalidAmount));
        assert!(codes.contains(&RecipeDiagnosticCode::MissingRequiredInput));
        assert!(codes.contains(&RecipeDiagnosticCode::DisconnectedNode));
    }

    #[test]
    fn rejects_reusing_a_consumed_wallet_asset() {
        let mut recipe = recipe();
        let second_trade = Uuid::from_u128(3);
        recipe.nodes.push(RecipeNode {
            id: second_trade,
            primitive: RecipePrimitive::Trade {
                buy_token: Address::with_last_byte(3),
            },
        });
        recipe.edges.push(RecipeEdge {
            id: Uuid::from_u128(4),
            from: RecipePortRef {
                node_id: SOURCE_ID,
                port: "asset".to_owned(),
            },
            to: RecipePortRef {
                node_id: second_trade,
                port: "sell".to_owned(),
            },
        });

        let compiled = compile(&recipe);

        assert!(
            compiled
                .report
                .diagnostics
                .iter()
                .any(|item| { item.code == RecipeDiagnosticCode::ConsumableOutputReused })
        );
    }

    #[test]
    fn compiles_trade_output_into_a_terminal_supply_dependency() {
        let mut recipe = recipe();
        recipe.nodes.push(RecipeNode {
            id: SUPPLY_ID,
            primitive: RecipePrimitive::Supply {
                venue: BorrowingVenue::Venus,
            },
        });
        recipe.edges.push(RecipeEdge {
            id: Uuid::from_u128(7),
            from: RecipePortRef {
                node_id: TRADE_ID,
                port: "buy".to_owned(),
            },
            to: RecipePortRef {
                node_id: SUPPLY_ID,
                port: "asset".to_owned(),
            },
        });

        let compiled = compile(&recipe);
        let plan = compiled
            .plan
            .unwrap_or_else(|| panic!("valid supply recipe did not produce a plan"));

        assert!(compiled.report.valid);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[1].dependencies, vec![TRADE_ID]);
        assert_eq!(
            plan.steps[1].action,
            RecipeAction::Supply {
                asset: AssetRef::NodeOutput {
                    node_id: TRADE_ID,
                    port: "buy".to_owned(),
                    token: Address::with_last_byte(2),
                },
                venue: BorrowingVenue::Venus,
            }
        );
    }

    #[test]
    fn compiles_supplied_position_into_terminal_withdrawal() {
        let pool = Address::with_last_byte(8);
        let token = Address::with_last_byte(9);
        let recipe = RecipeDefinition {
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
        };

        let compiled = compile(&recipe);
        let plan = compiled
            .plan
            .unwrap_or_else(|| panic!("valid withdrawal recipe did not produce a plan"));

        assert!(compiled.report.valid);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].node_id, WITHDRAW_ID);
        assert_eq!(
            plan.steps[0].action,
            RecipeAction::Withdraw {
                position: SuppliedPositionRef {
                    venue: BorrowingVenue::Venus,
                    pool,
                    token,
                },
            }
        );
    }

    #[test]
    fn compiles_supplied_position_into_borrow_with_exact_output() {
        let pool = Address::with_last_byte(8);
        let collateral = Address::with_last_byte(9);
        let debt = Address::with_last_byte(10);
        let amount = U256::from(1_000_000_u64);
        let recipe = RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: POSITION_ID,
                    primitive: RecipePrimitive::SuppliedPosition {
                        venue: BorrowingVenue::Venus,
                        pool,
                        token: collateral,
                    },
                },
                RecipeNode {
                    id: BORROW_ID,
                    primitive: RecipePrimitive::Borrow {
                        debt_token: debt,
                        raw_amount: amount,
                    },
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(13),
                from: RecipePortRef {
                    node_id: POSITION_ID,
                    port: "position".to_owned(),
                },
                to: RecipePortRef {
                    node_id: BORROW_ID,
                    port: "collateral".to_owned(),
                },
            }],
        };

        let compiled = compile(&recipe);
        let plan = compiled
            .plan
            .unwrap_or_else(|| panic!("valid borrow recipe did not produce a plan"));

        assert!(compiled.report.valid);
        assert_eq!(
            plan.steps[0].action,
            RecipeAction::Borrow {
                collateral: SuppliedPositionRef {
                    venue: BorrowingVenue::Venus,
                    pool,
                    token: collateral,
                },
                debt_token: debt,
                raw_amount: amount,
            }
        );
    }

    #[test]
    fn compiles_only_a_borrowed_position_into_repayment() {
        let pool = Address::with_last_byte(8);
        let debt = Address::with_last_byte(10);
        let mut recipe = RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: DEBT_ID,
                    primitive: RecipePrimitive::BorrowedPosition {
                        venue: BorrowingVenue::Venus,
                        pool,
                        token: debt,
                    },
                },
                RecipeNode {
                    id: REPAY_ID,
                    primitive: RecipePrimitive::Repay,
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(15),
                from: RecipePortRef {
                    node_id: DEBT_ID,
                    port: "debt".to_owned(),
                },
                to: RecipePortRef {
                    node_id: REPAY_ID,
                    port: "debt".to_owned(),
                },
            }],
        };

        let compiled = compile(&recipe);
        let plan = compiled
            .plan
            .unwrap_or_else(|| panic!("valid repayment recipe did not produce a plan"));
        assert!(compiled.report.valid);
        assert_eq!(
            plan.steps[0].action,
            RecipeAction::Repay {
                debt: BorrowedPositionRef {
                    venue: BorrowingVenue::Venus,
                    pool,
                    token: debt,
                },
            }
        );

        recipe.nodes[0].primitive = RecipePrimitive::SuppliedPosition {
            venue: BorrowingVenue::Venus,
            pool,
            token: debt,
        };
        recipe.edges[0].from.port = "position".to_owned();
        let rejected = compile(&recipe);
        assert!(rejected.plan.is_none());
        assert!(
            rejected
                .report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == RecipeDiagnosticCode::PortTypeMismatch)
        );
    }

    #[test]
    fn rejects_wallet_asset_connected_to_withdrawal() {
        let recipe = RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: SOURCE_ID,
                    primitive: RecipePrimitive::WalletAsset {
                        token: Address::with_last_byte(1),
                        raw_amount: U256::from(6),
                    },
                },
                RecipeNode {
                    id: WITHDRAW_ID,
                    primitive: RecipePrimitive::Withdraw,
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(11),
                from: RecipePortRef {
                    node_id: SOURCE_ID,
                    port: "asset".to_owned(),
                },
                to: RecipePortRef {
                    node_id: WITHDRAW_ID,
                    port: "position".to_owned(),
                },
            }],
        };

        let compiled = compile(&recipe);

        assert!(compiled.plan.is_none());
        assert!(
            compiled
                .report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == RecipeDiagnosticCode::PortTypeMismatch })
        );
    }

    fn recipe() -> RecipeDefinition {
        RecipeDefinition {
            schema_version: SCHEMA_VERSION,
            chain_id: BNB_CHAIN_ID,
            nodes: vec![
                RecipeNode {
                    id: SOURCE_ID,
                    primitive: RecipePrimitive::WalletAsset {
                        token: Address::with_last_byte(1),
                        raw_amount: U256::from(6),
                    },
                },
                RecipeNode {
                    id: TRADE_ID,
                    primitive: RecipePrimitive::Trade {
                        buy_token: Address::with_last_byte(2),
                    },
                },
            ],
            edges: vec![RecipeEdge {
                id: Uuid::from_u128(3),
                from: RecipePortRef {
                    node_id: SOURCE_ID,
                    port: "asset".to_owned(),
                },
                to: RecipePortRef {
                    node_id: TRADE_ID,
                    port: "sell".to_owned(),
                },
            }],
        }
    }
}
