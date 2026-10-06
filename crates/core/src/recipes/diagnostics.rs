//! Validation messages returned to the recipe editor.
//!
//! The compiler collects all problems it can find in one pass. Each diagnostic
//! has a stable code and a location, so the UI can highlight the exact node,
//! port, field, or edge that needs attention.
//!
//! Example: a trade with no incoming asset is reported at its `sell` port. The
//! editor can show that error beside the empty connector instead of displaying
//! one generic recipe error.

use serde::Serialize;
use uuid::Uuid;

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeDiagnosticCode {
    EmptyRecipe,
    UnsupportedSchemaVersion,
    UnsupportedChain,
    RecipeTooLarge,
    DuplicateNodeId,
    DuplicateEdgeId,
    UnknownNode,
    UnknownPort,
    InvalidPortDirection,
    PortTypeMismatch,
    MissingRequiredInput,
    TooManyConnections,
    ConsumableOutputReused,
    SelfConnection,
    RecipeCycle,
    DisconnectedNode,
    InvalidAmount,
    UnsupportedTradePair,
    UnsupportedSupplyAsset,
    UnsupportedWithdrawalPosition,
    UnsupportedBorrowPosition,
    UnsupportedBorrowAsset,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeDiagnosticLocation {
    Recipe,
    Node {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
    },
    Field {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        field: String,
    },
    Port {
        #[serde(rename = "nodeId")]
        node_id: Uuid,
        port: String,
    },
    Edge {
        #[serde(rename = "edgeId")]
        edge_id: Uuid,
    },
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDiagnostic {
    pub code: RecipeDiagnosticCode,
    pub severity: RecipeDiagnosticSeverity,
    pub location: RecipeDiagnosticLocation,
    pub message: String,
}

impl RecipeDiagnostic {
    pub(super) fn error(
        code: RecipeDiagnosticCode,
        location: RecipeDiagnosticLocation,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: RecipeDiagnosticSeverity::Error,
            location,
            message: message.into(),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeValidationReport {
    pub valid: bool,
    pub diagnostics: Vec<RecipeDiagnostic>,
}

impl RecipeValidationReport {
    pub(super) fn new(diagnostics: Vec<RecipeDiagnostic>) -> Self {
        let valid = diagnostics
            .iter()
            .all(|item| item.severity != RecipeDiagnosticSeverity::Error);
        Self { valid, diagnostics }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, to_value};

    use super::RecipeDiagnosticLocation;

    #[test]
    fn serializes_diagnostic_location_fields_as_camel_case() {
        let node_id = uuid::Uuid::from_u128(1);
        let result = to_value(RecipeDiagnosticLocation::Field {
            node_id,
            field: "rawAmount".to_owned(),
        });

        let Ok(value) = result else {
            panic!("recipe diagnostic location did not serialize");
        };
        assert_eq!(
            value,
            json!({
                "kind": "FIELD",
                "nodeId": node_id,
                "field": "rawAmount"
            })
        );
    }
}
