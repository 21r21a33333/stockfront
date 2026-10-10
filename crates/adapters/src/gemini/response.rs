use serde::Deserialize;
use stockfront_core::strategy_composer::{StrategyDraft, StrategyModelError};

use crate::strategy_prompt::draft_from_json;

#[derive(Deserialize)]
pub(super) struct GenerateContentResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
    /// Thinking models may return reasoning parts before the JSON answer.
    #[serde(default)]
    thought: bool,
}

impl TryFrom<GenerateContentResponse> for StrategyDraft {
    type Error = StrategyModelError;

    fn try_from(response: GenerateContentResponse) -> Result<Self, Self::Error> {
        let text = response
            .candidates
            .into_iter()
            .next()
            .and_then(|candidate| candidate.content)
            .map(|content| {
                content
                    .parts
                    .into_iter()
                    .filter(|part| !part.thought)
                    .filter_map(|part| part.text)
                    .collect::<String>()
            })
            .ok_or(StrategyModelError::InvalidResponse)?;
        draft_from_json(&text)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use stockfront_core::strategy_composer::StrategyStep;

    use super::*;

    #[test]
    fn parses_the_schema_constrained_answer_and_ignores_thought_parts() {
        let response = serde_json::from_value::<GenerateContentResponse>(serde_json::json!({
            "candidates": [{ "content": { "parts": [
                { "text": "reasoning", "thought": true },
                { "text": "{\"summary\":\"Buy Tesla\",\"steps\":[{\"kind\":\"WALLET_ASSET\",\"symbol\":\"USDT\",\"amount\":\"20.5\"},{\"kind\":\"TRADE\",\"buySymbol\":\"TSLAB\"}],\"assumptions\":[\"$20 means USDT\"]}" }
            ]}}]
        }))
        .unwrap_or_else(|error| panic!("fixture is invalid: {error}"));

        let draft = StrategyDraft::try_from(response)
            .unwrap_or_else(|error| panic!("valid answer rejected: {error}"));

        assert_eq!(
            draft.intent.steps,
            [
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

    #[test]
    fn numeric_amounts_and_empty_candidates_are_invalid_responses() {
        let numeric = serde_json::from_value::<GenerateContentResponse>(serde_json::json!({
            "candidates": [{ "content": { "parts": [
                { "text": "{\"summary\":\"x\",\"steps\":[{\"kind\":\"WALLET_ASSET\",\"symbol\":\"USDT\",\"amount\":20.1}]}" }
            ]}}]
        }))
        .unwrap_or_else(|error| panic!("fixture is invalid: {error}"));
        let empty = GenerateContentResponse {
            candidates: Vec::new(),
        };

        assert_eq!(
            StrategyDraft::try_from(numeric),
            Err(StrategyModelError::InvalidResponse)
        );
        assert_eq!(
            StrategyDraft::try_from(empty),
            Err(StrategyModelError::InvalidResponse)
        );
    }
}
