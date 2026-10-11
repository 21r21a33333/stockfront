//! Provider-neutral prompt, schema and draft payload for the strategy composer.
//!
//! The wording of the instructions and the shape of the answer are product
//! decisions, not vendor details, so every model adapter shares them.

use std::fmt::Write as _;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};
use stockfront_core::strategy_composer::{
    ComposeContext, ComposerAssetKind, ComposerPositionKind, HOUSE_STRATEGIES, StrategyDraft,
    StrategyIntent, StrategyModelError, StrategyStep,
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
- TRADE {buySymbol}: swaps the incoming tokens. The input must be tokens, from a WALLET_ASSET or a BORROW; a supplied or borrowed position can never feed a TRADE. Outputs the bought tokens.
- SUPPLY: supplies the incoming stock to Venus as collateral. Only stocks. Outputs the new collateral position, which a BORROW or WITHDRAW may use.
- EARN_DEPOSIT {symbol}: deposits the incoming stablecoin into a reviewed earn product. symbol is the stock the earn rate is compared against, so use the collateral behind the loan that funded it. Stockfront picks the product. Only stablecoins. Ends the chain.
- BORROW {symbol, amount}: input must be a supplied position, either one listed below or one a SUPPLY step earlier in the same chain just created; borrows a borrowable stablecoin. Outputs the borrowed tokens.
- REPAY: input must be a borrowed position; repays the whole debt. Ends the chain.
- WITHDRAW: input must be a supplied position; withdraws all of it, only possible with no debt. Ends the chain.

Valid examples:
- WALLET_ASSET(USDT, 20) -> TRADE(TSLAB) -> SUPPLY
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> TRADE(NVDAB)
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> TRADE(TSLAB) -> SUPPLY
- SUPPLIED_POSITION(TSLAB) -> BORROW(USDT, 10) -> EARN_DEPOSIT(TSLAB)
- WALLET_ASSET(USDT, 20) -> TRADE(TSLAB) -> SUPPLY -> BORROW(USDT, 8) -> EARN_DEPOSIT(TSLAB)
- WALLET_ASSET(USDT, 20) -> TRADE(TSLAB) -> SUPPLY -> BORROW(USDT, 8) -> TRADE(TSLAB) -> SUPPLY
- BORROWED_POSITION(USDT) -> REPAY
- SUPPLIED_POSITION(TSLAB) -> WITHDRAW

Rules:
- Pick the source from what the user holds, not from the verb. Tokens the user owns are WALLET_ASSET. Use SUPPLIED_POSITION or BORROWED_POSITION only for a position that appears in the position list below; when that list says none, you must not use them at all.
- Selling, swapping back or cashing out a stock means TRADE into a stablecoin starting from WALLET_ASSET, unless the user says the stock is supplied or used as collateral. Withdrawing, unsupplying or taking back collateral means WITHDRAW from a supplied position.
- Use only symbols from the asset list. Map company names and tickers to their bStock symbol.
- A dollar amount means that many USDT; say so in assumptions.
- Amounts are decimal strings in token units. Never invent an amount.
- How much may be borrowed depends on live collateral prices and Venus risk limits that are not shown here. \
Never estimate one. If the user asks to borrow the most they can, as much as possible, or the maximum, \
set maximum to true and leave amount null; Stockfront reads the safe limit from Venus when the loan is signed. \
Otherwise set maximum to false and give the amount the user asked for.
- All, everything, my whole or my entire holding of an asset means exactly the amount shown beside it below. \
Use null only when the user gave no amount and no holding is shown for that asset.
- A chain may SUPPLY a stock and then BORROW against that same new supply. Every step is prepared only after the step before it has executed, \
so that borrow is sized against the collateral that actually arrived.
- Earning yield on a stablecoin means EARN_DEPOSIT. Leverage means repeating BORROW -> TRADE -> SUPPLY; write every turn out as explicit steps, and use at most three turns. \
Liquidity provision, perpetuals, other chains and unlisted assets are not supported yet: \
return no steps and explain why in assumptions.
- summary is one plain-English sentence. assumptions are short sentences.";

/// What a TRADE may move between, which the deployment can widen.
const STOCK_PAIRS_ONLY: &str =
    "A TRADE goes stablecoin to stock, or stock to stablecoin. Never stablecoin to stablecoin.";
/// What a basket is, told only when the catalog has any.
const BASKETS: &str = "A basket is an index token holding many assets at once. Use one when the \
user names an index or asks for broad exposure rather than a single company. A basket is only \
ever traded: it cannot be supplied, borrowed against or deposited into earn, so a basket TRADE \
always ends the chain.";

const SETTLEMENT_PAIRS: &str = "A TRADE goes stablecoin to stock, stock to stablecoin, or one \
stablecoin to another. Use a stablecoin-to-stablecoin TRADE to convert between stablecoins \
directly; never route one through a stock to do it.";

/// The strategies Stockfront offers, so a request for "a strategy" or "the best
/// return" lands on a shape the product actually supports.
fn strategies() -> String {
    let mut text = String::from(
        "\nStrategies Stockfront offers. When the user asks for a strategy, the best return, \
income, or yield rather than one exact action, build one of these shapes using whichever \
stock or basket suits what they asked for:\n",
    );
    for strategy in HOUSE_STRATEGIES {
        let _ = writeln!(
            text,
            "- {} ({}): {}\n  Use when the user wants {}",
            strategy.name,
            strategy.risk.label(),
            strategy.chain,
            strategy.when
        );
    }
    text.push_str(
        "Pick the one strategy that matches the risk the user accepted, build it, and name the \
others you considered in assumptions. Never return more than one strategy.\n",
    );
    text
}

pub(crate) fn instructions(context: &ComposeContext) -> String {
    let pairs = if context.settlement_pairs {
        SETTLEMENT_PAIRS
    } else {
        STOCK_PAIRS_ONLY
    };
    let mut text = format!("{INSTRUCTIONS}\n{pairs}\n");
    if context
        .assets
        .iter()
        .any(|asset| asset.kind == ComposerAssetKind::Basket)
    {
        let _ = writeln!(text, "{BASKETS}");
    }
    if !context.asked.is_empty() {
        let _ = writeln!(
            text,
            "\nYou asked the user: {}\nTheir message is the answer. Apply it to the strategy \
being refined and return that strategy complete, changing nothing else.",
            context.asked.join(" and ")
        );
    }
    text.push_str(&strategies());
    // Holdings are known only with a wallet connected; without them "all of it"
    // has no number and the model has to ask.
    let holdings_known = context.assets.iter().any(|asset| asset.balance.is_some());
    text.push_str("\nAssets:\n");
    for asset in &context.assets {
        let kind = match asset.kind {
            ComposerAssetKind::Stock => "stock",
            ComposerAssetKind::Basket => "index basket",
            ComposerAssetKind::Stablecoin { borrowable: true } => "stablecoin, borrowable",
            ComposerAssetKind::Stablecoin { borrowable: false } => "stablecoin",
        };
        let held = match asset.balance {
            Some(balance) if balance > Decimal::ZERO => {
                format!(", you hold {}", balance.normalize())
            }
            _ => String::new(),
        };
        let _ = writeln!(text, "- {}: {} ({kind}{held})", asset.symbol, asset.name);
        if let Some(detail) = asset.detail.as_deref().filter(|detail| !detail.is_empty()) {
            let _ = writeln!(text, "    {detail}");
        }
    }
    if holdings_known {
        text.push_str(
            "Every holding is shown above; an asset with no holding is one you do not own.\n",
        );
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
    let maximum = json!({
        "type": "boolean",
        "description": "True when the user asked to borrow the most they can; then amount is null."
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
                    step(
                        "BORROW",
                        json!({ "symbol": symbol, "amount": amount, "maximum": maximum }),
                    ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use stockfront_core::strategy_composer::ComposerAsset;

    fn context(assets: Vec<ComposerAsset>) -> ComposeContext {
        ComposeContext {
            prompt: "put my stock to work".to_owned(),
            settlement_pairs: false,
            assets,
            positions: Vec::new(),
            previous: None,
            asked: Vec::new(),
            rejected: None,
        }
    }

    fn asset(symbol: &str, balance: Option<&str>, detail: Option<&str>) -> ComposerAsset {
        ComposerAsset {
            symbol: symbol.to_owned(),
            name: "Tesla, Inc.".to_owned(),
            kind: ComposerAssetKind::Stock,
            balance: balance.map(|value| value.parse().expect("test balance is a decimal")),
            detail: detail.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn a_held_asset_carries_its_amount_so_all_of_it_needs_no_question() {
        let text = instructions(&context(vec![asset("TSLAB", Some("0.018270"), None)]));

        // Trailing zeros would read as precision the balance does not have.
        assert!(text.contains("- TSLAB: Tesla, Inc. (stock, you hold 0.01827)"));
        assert!(text.contains("an asset with no holding is one you do not own"));
    }

    #[test]
    fn an_asset_held_at_zero_reads_the_same_as_one_not_held() {
        let text = instructions(&context(vec![
            asset("TSLAB", Some("0"), None),
            asset("NVDAB", None, None),
        ]));

        assert!(text.contains("- TSLAB: Tesla, Inc. (stock)\n"));
        assert!(text.contains("- NVDAB: Tesla, Inc. (stock)\n"));
    }

    #[test]
    fn detail_rides_under_the_asset_it_describes() {
        let text = instructions(&context(vec![asset(
            "CMC20",
            None,
            Some("Themes: Majors, DeFi. Holds BTC 30%."),
        )]));

        assert!(text.contains("- CMC20: Tesla, Inc. (stock)\n    Themes: Majors, DeFi."));
    }

    #[test]
    fn every_house_strategy_is_offered_with_its_shape_and_risk() {
        let text = instructions(&context(Vec::new()));

        for strategy in HOUSE_STRATEGIES {
            assert!(text.contains(strategy.name), "{} missing", strategy.name);
            assert!(
                text.contains(strategy.chain),
                "{} shape missing",
                strategy.name
            );
        }
        assert!(text.contains("Never return more than one strategy."));
    }
}
