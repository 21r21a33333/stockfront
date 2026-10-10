use std::{env, time::Duration};

use rust_decimal::Decimal;
use stockfront_adapters::openai_compatible::OpenAiCompatibleClient;
use stockfront_core::strategy_composer::{
    ComposeContext, ComposerAsset, ComposerAssetKind, StrategyModel, StrategyStep,
};

#[tokio::test]
#[ignore = "calls the live OpenRouter API; requires OPENROUTER_API_KEY"]
async fn drafts_a_buy_and_supply_chain_from_plain_english() {
    let api_key = env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|error| panic!("OPENROUTER_API_KEY is required: {error}"));
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_owned());
    let model =
        env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "google/gemini-2.5-flash".to_owned());
    let require_parameters =
        env::var("OPENROUTER_REQUIRE_PARAMETERS").is_ok_and(|value| value == "true");
    let client = OpenAiCompatibleClient::new(
        &base_url,
        &model,
        &api_key,
        require_parameters,
        Duration::from_mins(1),
    )
    .unwrap_or_else(|error| panic!("OpenRouter client was not configured: {error}"));
    let context = ComposeContext {
        settlement_pairs: false,
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
        .unwrap_or_else(|error| panic!("OpenRouter draft failed: {error}"));

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
