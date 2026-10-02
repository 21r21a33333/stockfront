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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipeDiagnosticLocation {
    Recipe,
    Node { node_id: Uuid },
    Field { node_id: Uuid, field: String },
    Port { node_id: Uuid, port: String },
    Edge { edge_id: Uuid },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
