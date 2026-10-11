use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;
use stockfront_core::strategy_composer::ComposeContext;

use crate::strategy_prompt::{instructions, intent_json, response_schema};

#[derive(Serialize)]
pub(super) struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    temperature: u8,
    response_format: ResponseFormat,
    /// `OpenRouter` only: refuses providers that would drop `response_format`,
    /// which otherwise returns prose instead of the requested JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<ProviderPreferences>,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    json_schema: JsonSchema,
}

#[derive(Serialize)]
struct JsonSchema {
    name: &'static str,
    strict: bool,
    schema: Value,
}

#[derive(Serialize)]
struct ProviderPreferences {
    require_parameters: bool,
}

impl ChatCompletionRequest {
    pub(super) fn new(model: String, require_parameters: bool, context: &ComposeContext) -> Self {
        let mut user = context.prompt.clone();
        if let Some(previous) = &context.previous {
            let _ = write!(
                user,
                "\n\nCurrent strategy to refine: {}",
                intent_json(previous)
            );
        }
        if let Some(rejected) = &context.rejected {
            let _ = write!(
                user,
                "\n\nYour previous draft {} was rejected:\n- {}\nReturn a corrected strategy.",
                intent_json(&rejected.intent),
                rejected.issues.join("\n- ")
            );
        }
        Self {
            model,
            messages: vec![
                Message {
                    role: "system",
                    content: instructions(context),
                },
                Message {
                    role: "user",
                    content: user,
                },
            ],
            temperature: 0,
            response_format: ResponseFormat {
                kind: "json_schema",
                json_schema: JsonSchema {
                    name: "stockfront_strategy",
                    strict: true,
                    schema: response_schema(),
                },
            },
            provider: require_parameters.then_some(ProviderPreferences {
                require_parameters: true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use stockfront_core::strategy_composer::{
        ComposerAsset, ComposerAssetKind, RejectedDraft, StrategyIntent, StrategyStep,
    };

    use super::*;

    fn context() -> ComposeContext {
        ComposeContext {
            settlement_pairs: false,
            prompt: "buy tesla".to_owned(),
            assets: vec![ComposerAsset {
                symbol: "TSLAB".to_owned(),
                name: "Tesla".to_owned(),
                kind: ComposerAssetKind::Stock { collateral: true },
                balance: None,
                detail: None,
            }],
            positions: Vec::new(),
            previous: None,
            asked: Vec::new(),
            rejected: Some(RejectedDraft {
                intent: StrategyIntent {
                    steps: vec![StrategyStep::Supply],
                },
                issues: vec!["Step 1 must be a wallet asset".to_owned()],
            }),
        }
    }

    #[test]
    fn schema_is_sent_strictly_and_pinned_to_a_capable_provider() {
        let body = serde_json::to_value(ChatCompletionRequest::new(
            "openai/gpt-4o-mini".to_owned(),
            true,
            &context(),
        ))
        .unwrap_or_else(|error| panic!("request did not serialize: {error}"));

        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert!(
            body["response_format"]["json_schema"]["schema"]["properties"]["steps"].is_object()
        );
        assert_eq!(body["provider"]["require_parameters"], true);
    }

    #[test]
    fn provider_pinning_is_omitted_for_servers_that_reject_the_field() {
        let body = serde_json::to_value(ChatCompletionRequest::new(
            "llama".to_owned(),
            false,
            &context(),
        ))
        .unwrap_or_else(|error| panic!("request did not serialize: {error}"));

        assert!(body.get("provider").is_none());
    }

    #[test]
    fn repair_requests_carry_the_rejected_draft_and_its_issues() {
        let body = serde_json::to_value(ChatCompletionRequest::new(
            "openai/gpt-4o-mini".to_owned(),
            true,
            &context(),
        ))
        .unwrap_or_else(|error| panic!("request did not serialize: {error}"));

        assert!(
            body["messages"][0]["content"]
                .as_str()
                .is_some_and(|text| text.contains("- TSLAB: Tesla (stock, usable as collateral)"))
        );
        assert!(
            body["messages"][1]["content"]
                .as_str()
                .is_some_and(|text| text.contains("SUPPLY") && text.contains("Step 1"))
        );
    }
}
