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
    AssetRef, LogicalPlan, LogicalStep, RecipeAction, RecipeDefinition, RecipeDiagnostic,
    RecipeDiagnosticCode, RecipeDiagnosticLocation, RecipeGraph, RecipePrimitive,
    RecipeValidationReport,
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
        if let RecipePrimitive::WalletAsset { raw_amount, .. } = &node.primitive
            && raw_amount.is_zero()
        {
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
            RecipePrimitive::Trade { buy_token } => {
                let Some(edge) = graph.incoming(node_id, "sell") else {
                    return Err(missing_input(node_id, "sell"));
                };
                let source_key = (edge.edge.from.node_id, edge.edge.from.port.clone());
                let sell = values.get(&source_key).cloned().ok_or_else(|| {
                    RecipeDiagnostic::error(
                        RecipeDiagnosticCode::MissingRequiredInput,
                        RecipeDiagnosticLocation::Port {
                            node_id,
                            port: "sell".to_owned(),
                        },
                        "the trade input is not available before this step",
                    )
                })?;
                let dependency = edge.edge.from.node_id;
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
                let Some(edge) = graph.incoming(node_id, "asset") else {
                    return Err(missing_input(node_id, "asset"));
                };
                let source_key = (edge.edge.from.node_id, edge.edge.from.port.clone());
                let asset = values.get(&source_key).cloned().ok_or_else(|| {
                    RecipeDiagnostic::error(
                        RecipeDiagnosticCode::MissingRequiredInput,
                        RecipeDiagnosticLocation::Port {
                            node_id,
                            port: "asset".to_owned(),
                        },
                        "the supply input is not available before this step",
                    )
                })?;
                let dependency = edge.edge.from.node_id;
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
        }
    }
    Ok(LogicalPlan { steps })
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
