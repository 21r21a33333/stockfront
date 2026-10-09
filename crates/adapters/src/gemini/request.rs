use std::fmt::Write as _;

use serde::Serialize;
use serde_json::{Value, json};
use stockfront_core::strategy_composer::{
    ComposeContext, ComposerAssetKind, ComposerPositionKind, StrategyIntent,
};

const INSTRUCTIONS: &str = "\
You design strategies for Stockfront, a non-custodial app for tokenized stocks (bStocks) on BNB Chain. \
Translate the user's request into the JSON schema. You only propose; the user reviews and signs every step.

A strategy is an ordered chain. The first step is exactly one source; every later step consumes the previous step's output.
Sources:
- WALLET_ASSET {symbol, amount}: tokens from the user's wallet. Outputs those tokens.
- SUPPLIED_POSITION {symbol}: a stock the user already supplied to Venus. Use only positions listed below. Outputs the position.
- BORROWED_POSITION {symbol}: an existing Venus debt listed below. Outputs the debt.
Actions:
- TRADE {buySymbol}: swaps the incoming tokens. Only stablecoin to stock, or stock to stablecoin. Outputs the bought tokens.
- SUPPLY: supplies the incoming stock to Venus as collateral. Only stocks. Ends the chain.
- BORROW {symbol, amount}: input must be a supplied position; borrows a borrowable stablecoin. Outputs the borrowed tokens.
- REPAY: input must be a borrowed position; repays the whole debt. Ends the chain.
- WITHDRAW: input must be a supplied position; withdraws all of it, only possible with no debt. Ends the chain.

Valid examples:
- WALLET_ASSET(USDT, 20) -> TRADE(TSLAB) -> SUPPLY
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> TRADE(NVDAB)
- BORROWED_POSITION(USDT) -> REPAY
- SUPPLIED_POSITION(TSLAB) -> WITHDRAW

Rules:
- Use only symbols from the asset list. Map company names and tickers to their bStock symbol.
- A dollar amount means that many USDT; say so in assumptions.
- Amounts are decimal strings in token units. Never invent an amount: use null when the user gave none.
- Borrowing needs a position that is already supplied, so buying, supplying and borrowing cannot be one chain. \
Produce the first chain and explain the follow-up in assumptions.
- Earning yield, liquidity provision, leverage loops, perpetuals, other chains and unlisted assets are not supported yet: \
return no steps and explain why in assumptions.
- summary is one plain-English sentence. assumptions are short sentences.";

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

fn instructions(context: &ComposeContext) -> String {
    let mut text = format!("{INSTRUCTIONS}\n\nAssets:\n");
    for asset in &context.assets {
        let kind = match asset.kind {
            ComposerAssetKind::Stock => "stock",
            ComposerAssetKind::Stablecoin { borrowable: true } => "stablecoin, borrowable",
            ComposerAssetKind::Stablecoin { borrowable: false } => "stablecoin",
        };
        let _ = writeln!(text, "- {}: {} ({kind})", asset.symbol, asset.name);
    }
    text.push_str("\nWallet positions on Venus:\n");
    if context.positions.is_empty() {
        text.push_str("- none\n");
    }
    for position in &context.positions {
        let kind = match position.kind {
            ComposerPositionKind::Supplied => "supplied",
            ComposerPositionKind::Borrowed => "borrowed",
        };
        let _ = writeln!(text, "- {kind} {} {}", position.amount, position.symbol);
    }
    text
}

fn intent_json(intent: &StrategyIntent) -> String {
    serde_json::to_string(intent).unwrap_or_default()
}

fn response_schema() -> Value {
    let symbol = json!({ "type": "string", "description": "An asset symbol from the asset list." });
    let amount = json!({
        "type": ["string", "null"],
        "description": "Decimal token amount such as \"20\" or \"0.5\", or null when the user gave none."
    });
    let step = |kind: &str, fields: Value| {
        let mut properties = json!({ "kind": { "type": "string", "enum": [kind] } });
        let mut required = vec![json!("kind")];
        if let (Some(target), Some(fields)) = (properties.as_object_mut(), fields.as_object()) {
            for (name, schema) in fields {
                target.insert(name.clone(), schema.clone());
                required.push(json!(name));
            }
        }
        json!({ "type": "object", "properties": properties, "required": required })
    };
    json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "steps": {
                "type": "array",
                "items": { "anyOf": [
                    step("WALLET_ASSET", json!({ "symbol": symbol, "amount": amount })),
                    step("SUPPLIED_POSITION", json!({ "symbol": symbol })),
                    step("BORROWED_POSITION", json!({ "symbol": symbol })),
                    step("TRADE", json!({ "buySymbol": symbol })),
                    step("SUPPLY", json!({})),
                    step("BORROW", json!({ "symbol": symbol, "amount": amount })),
                    step("REPAY", json!({})),
                    step("WITHDRAW", json!({})),
                ]}
            },
            "assumptions": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["summary", "steps", "assumptions"]
    })
}

#[cfg(test)]
mod tests {
    use stockfront_core::strategy_composer::{ComposerAsset, RejectedDraft, StrategyStep};

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
