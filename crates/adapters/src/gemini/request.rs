use serde::Serialize;
use serde_json::Value;
use stockfront_core::strategy_composer::ComposeContext;

use crate::strategy_prompt::{instructions, intent_json, response_schema};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GenerateContentRequest {
    system_instruction: Content,
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    response_mime_type: &'static str,
    response_json_schema: Value,
    /// Deterministic drafts make repairs and demos reproducible.
    temperature: u8,
}

impl From<&ComposeContext> for GenerateContentRequest {
    fn from(context: &ComposeContext) -> Self {
        let mut user = vec![Part {
            text: context.prompt.clone(),
        }];
        if let Some(previous) = &context.previous {
            user.push(Part {
                text: format!("Current strategy to refine: {}", intent_json(previous)),
            });
        }
        if let Some(rejected) = &context.rejected {
            user.push(Part {
                text: format!(
                    "Your previous draft {} was rejected:\n- {}\nReturn a corrected strategy.",
                    intent_json(&rejected.intent),
                    rejected.issues.join("\n- ")
                ),
            });
        }
        Self {
            system_instruction: Content {
                role: None,
                parts: vec![Part {
                    text: instructions(context),
                }],
            },
            contents: vec![Content {
                role: Some("user"),
                parts: user,
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json",
                response_json_schema: response_schema(),
                temperature: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use stockfront_core::strategy_composer::{
        ComposerAsset, ComposerAssetKind, RejectedDraft, StrategyIntent, StrategyStep,
    };

    use super::*;

    #[test]
    fn repair_requests_carry_the_rejected_draft_and_its_issues() {
        let context = ComposeContext {
            prompt: "buy tesla".to_owned(),
            assets: vec![ComposerAsset {
                symbol: "TSLAB".to_owned(),
                name: "Tesla".to_owned(),
                kind: ComposerAssetKind::Stock,
            }],
            positions: Vec::new(),
            previous: None,
            rejected: Some(RejectedDraft {
                intent: StrategyIntent {
                    steps: vec![StrategyStep::Supply],
                },
                issues: vec!["Step 1 must be a wallet asset".to_owned()],
            }),
        };

        let body = serde_json::to_value(GenerateContentRequest::from(&context))
            .unwrap_or_else(|error| panic!("request did not serialize: {error}"));

        let parts = &body["contents"][0]["parts"];
        assert_eq!(parts[0]["text"], "buy tesla");
        assert!(
            parts[1]["text"]
                .as_str()
                .is_some_and(|text| text.contains("SUPPLY") && text.contains("Step 1"))
        );
        assert!(
            body["systemInstruction"]["parts"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("- TSLAB: Tesla (stock)"))
        );
        assert_eq!(
            body["generationConfig"]["responseMimeType"],
            "application/json"
        );
    }
}
