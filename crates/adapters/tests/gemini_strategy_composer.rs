use std::{env, time::Duration};

use rust_decimal::Decimal;
use stockfront_adapters::gemini::GeminiClient;
use stockfront_core::strategy_composer::{
    ComposeContext, ComposerAsset, ComposerAssetKind, StrategyModel, StrategyStep,
};

#[tokio::test]
#[ignore = "calls the live Gemini API; requires GEMINI_API_KEY"]
async fn drafts_a_buy_and_supply_chain_from_plain_english() {
    let api_key = env::var("GEMINI_API_KEY")
        .unwrap_or_else(|error| panic!("GEMINI_API_KEY is required: {error}"));
    let model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-flash-latest".to_owned());
    let client = GeminiClient::new(
        "https://generativelanguage.googleapis.com/v1beta",
        &model,
        &api_key,
        Duration::from_secs(60),
    )
    .unwrap_or_else(|error| panic!("Gemini client was not configured: {error}"));
    let context = ComposeContext {
        prompt: "Swap 6 USDT for Nvidia stock and use it as collateral on Venus".to_owned(),
        assets: vec![
            asset(
                "USDT",
                "USDT",
                ComposerAssetKind::Stablecoin { borrowable: true },
            ),
            asset("NVDAB", "NVIDIA", ComposerAssetKind::Stock),
            asset("TSLAB", "Tesla", ComposerAssetKind::Stock),
        ],
        positions: Vec::new(),
        previous: None,
        rejected: None,
    };

    let draft = client
        .draft(&context)
        .await
        .unwrap_or_else(|error| panic!("Gemini draft failed: {error}"));

    assert_eq!(
        draft.intent.steps,
        [
            StrategyStep::WalletAsset {
                symbol: "USDT".to_owned(),
                amount: Some(Decimal::from(6)),
            },
            StrategyStep::Trade {
                buy_symbol: "NVDAB".to_owned(),
            },
            StrategyStep::Supply,
        ]
    );
    assert!(!draft.summary.trim().is_empty());
}

fn asset(symbol: &str, name: &str, kind: ComposerAssetKind) -> ComposerAsset {
    ComposerAsset {
        symbol: symbol.to_owned(),
        name: name.to_owned(),
        kind,
    }
}
