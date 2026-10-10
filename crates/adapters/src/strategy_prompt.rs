//! Provider-neutral prompt, schema and draft payload for the strategy composer.
//!
//! The wording of the instructions and the shape of the answer are product
//! decisions, not vendor details, so every model adapter shares them.

use std::fmt::Write as _;

use serde::Deserialize;
use serde_json::{Value, json};
use stockfront_core::strategy_composer::{
    ComposeContext, ComposerAssetKind, ComposerPositionKind, StrategyDraft, StrategyIntent,
    StrategyModelError, StrategyStep,
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
- TRADE {buySymbol}: swaps the incoming tokens. The input must be tokens, from a WALLET_ASSET or a BORROW; a supplied or borrowed position can never feed a TRADE. Only stablecoin to stock, or stock to stablecoin. Outputs the bought tokens.
- SUPPLY: supplies the incoming stock to Venus as collateral. Only stocks. Ends the chain.
- EARN_DEPOSIT {symbol}: deposits the incoming stablecoin into a reviewed earn product. symbol is the stock the earn rate is compared against, so use the collateral behind the loan that funded it. Stockfront picks the product. Only stablecoins. Ends the chain.
- BORROW {symbol, amount}: input must be a supplied position; borrows a borrowable stablecoin. Outputs the borrowed tokens.
- REPAY: input must be a borrowed position; repays the whole debt. Ends the chain.
- WITHDRAW: input must be a supplied position; withdraws all of it, only possible with no debt. Ends the chain.

Valid examples:
- WALLET_ASSET(USDT, 20) -> TRADE(TSLAB) -> SUPPLY
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> TRADE(NVDAB)
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> TRADE(TSLAB) -> SUPPLY
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> EARN_DEPOSIT(TSLAB)
- BORROWED_POSITION(USDT) -> REPAY
- SUPPLIED_POSITION(TSLAB) -> WITHDRAW

Rules:
- Pick the source from what the user holds, not from the verb. Tokens the user owns are WALLET_ASSET. Use SUPPLIED_POSITION or BORROWED_POSITION only for a position that appears in the position list below; when that list says none, you must not use them at all.
- Selling, swapping back or cashing out a stock means TRADE into a stablecoin starting from WALLET_ASSET, unless the user says the stock is supplied or used as collateral. Withdrawing, unsupplying or taking back collateral means WITHDRAW from a supplied position.
- Use only symbols from the asset list. Map company names and tickers to their bStock symbol.
- A dollar amount means that many USDT; say so in assumptions.
- Amounts are decimal strings in token units. Never invent an amount: use null when the user gave none.
- BORROW needs a position that is already supplied, so one chain cannot SUPPLY a stock and then BORROW against that same new supply. \
Steps after a BORROW are fine: BORROW may be followed by TRADE, and that TRADE by SUPPLY. \
Only when a supply must happen first, produce the first chain and explain the follow-up in assumptions.
- Earning yield on a stablecoin means EARN_DEPOSIT. Liquidity provision, repeated or automated leverage loops, perpetuals, other chains and unlisted assets are not supported yet: \
return no steps and explain why in assumptions.
- summary is one plain-English sentence. assumptions are short sentences.";

pub(crate) fn instructions(context: &ComposeContext) -> String {
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

pub(crate) fn intent_json(intent: &StrategyIntent) -> String {
    serde_json::to_string(intent).unwrap_or_default()
}

/// `OpenAI`-family models reject a strict schema whose objects omit
/// `additionalProperties: false`, so every object closes itself.
pub(crate) fn response_schema() -> Value {
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
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        })
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
                    step("EARN_DEPOSIT", json!({ "symbol": symbol })),
                ]}
            },
            "assumptions": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["summary", "steps", "assumptions"],
        "additionalProperties": false
    })
}

/// The JSON document constrained by [`response_schema`].
#[derive(Deserialize)]
pub(crate) struct DraftPayload {
    summary: String,
    steps: Vec<StrategyStep>,
    #[serde(default)]
    assumptions: Vec<String>,
}

/// Parses a model's JSON answer into a draft.
///
/// # Errors
///
/// Returns [`StrategyModelError::InvalidResponse`] when the text is empty or
/// does not match the requested schema.
pub(crate) fn draft_from_json(text: &str) -> Result<StrategyDraft, StrategyModelError> {
    if text.trim().is_empty() {
        return Err(StrategyModelError::InvalidResponse);
    }
    let payload = serde_json::from_str::<DraftPayload>(text)
        .map_err(|_| StrategyModelError::InvalidResponse)?;
    Ok(StrategyDraft {
        summary: payload.summary,
        intent: StrategyIntent {
            steps: payload.steps,
        },
        assumptions: payload.assumptions,
    })
}
