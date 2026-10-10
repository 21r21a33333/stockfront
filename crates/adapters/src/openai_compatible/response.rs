use serde::Deserialize;
use stockfront_core::strategy_composer::{StrategyDraft, StrategyModelError};

use crate::strategy_prompt::draft_from_json;

#[derive(Deserialize)]
pub(super) struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ChoiceMessage>,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

impl TryFrom<ChatCompletionResponse> for StrategyDraft {
    type Error = StrategyModelError;

    fn try_from(response: ChatCompletionResponse) -> Result<Self, Self::Error> {
        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message)
            .and_then(|message| message.content)
            .ok_or(StrategyModelError::InvalidResponse)?;
        draft_from_json(&content)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use stockfront_core::strategy_composer::StrategyStep;

    use super::*;

    fn parse(body: serde_json::Value) -> Result<StrategyDraft, StrategyModelError> {
        serde_json::from_value::<ChatCompletionResponse>(body)
            .unwrap_or_else(|error| panic!("fixture did not deserialize: {error}"))
            .try_into()
    }

    #[test]
    fn reads_the_strategy_from_the_first_choice() {
        let draft = parse(serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content":
                "{\"summary\":\"Buy Tesla\",\"steps\":[{\"kind\":\"WALLET_ASSET\",\"symbol\":\"USDT\",\"amount\":\"20.5\"},{\"kind\":\"TRADE\",\"buySymbol\":\"TSLAB\"}],\"assumptions\":[\"$20 means USDT\"]}"
            }}]
        }))
        .unwrap_or_else(|error| panic!("draft was rejected: {error}"));

        assert_eq!(draft.summary, "Buy Tesla");
        assert_eq!(draft.assumptions, vec!["$20 means USDT".to_owned()]);
        assert_eq!(
            draft.intent.steps,
            vec![
                StrategyStep::WalletAsset {
                    symbol: "USDT".to_owned(),
                    amount: Some(Decimal::new(205, 1)),
                },
                StrategyStep::Trade {
                    buy_symbol: "TSLAB".to_owned(),
                },
            ]
        );
    }

    /// A provider that ignores `response_format` answers with prose, and an
    /// overloaded one answers with no choices at all.
    #[test]
    fn prose_and_empty_choices_are_invalid_responses() {
        assert!(matches!(
            parse(serde_json::json!({
                "choices": [{ "message": { "content": "Sure! Here is a strategy for you." }}]
            })),
            Err(StrategyModelError::InvalidResponse)
        ));
        assert!(matches!(
            parse(serde_json::json!({ "choices": [] })),
            Err(StrategyModelError::InvalidResponse)
        ));
    }
}
