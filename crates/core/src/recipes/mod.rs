//! Recipe language, compiler, and provider-backed preparation service.
//!
//! The kernel is intentionally split into three layers:
//!
//! - `model`, `graph`, and `compiler` define the saved language and pure rules.
//! - `plan` is the internal instruction set produced by the compiler.
//! - `service` resolves logical actions into live quotes and prepared actions.
//!
//! Provider details must not move into the compiler. Adding a new provider or
//! changing a quote endpoint should not change whether a recipe is valid.

mod compiler;
mod diagnostics;
mod graph;
mod model;
mod plan;
mod service;

use compiler::compile;
use graph::RecipeGraph;
use plan::{AssetRef, LogicalPlan, LogicalStep, RecipeAction};

pub use compiler::SCHEMA_VERSION;
pub use diagnostics::{
    RecipeDiagnostic, RecipeDiagnosticCode, RecipeDiagnosticLocation, RecipeDiagnosticSeverity,
    RecipeValidationReport,
};
pub use model::{
    PrepareRecipeRequest, PreparedRecipe, PreparedRecipeStep, RecipeCatalog, RecipeContext,
    RecipeDefinition, RecipeEdge, RecipeFieldControl, RecipeFieldDescriptor, RecipeNode,
    RecipeNodePreview, RecipeNodeSelection, RecipeNodeState, RecipeOptionSource,
    RecipePortDescriptor, RecipePortDirection, RecipePortFlow, RecipePortRef, RecipePortValueType,
    RecipePreview, RecipePreviewRequest, RecipePrimitive, RecipePrimitiveDescriptor,
    RecipePrimitiveKind,
};
pub use service::{RecipeError, RecipeService};
