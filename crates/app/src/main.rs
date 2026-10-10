mod setting;

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use setting::{AiProvider, AiSettings, Settings};
use stockfront_adapters::{
    binance_web3::BinanceWeb3Client,
    gemini::GeminiClient,
    glider::GliderAdapter,
    http::{
        api, basket_router, borrowing_router, bstock_router, carry_router, liquidity_router,
        managed_portfolio_router, recipe_router, strategy_router, trade_router, wallet_router,
    },
    openai_compatible::OpenAiCompatibleClient,
    reserve::ReserveAdapter,
    venus::VenusBorrowingAdapter,
};
use stockfront_core::{
    baskets::BasketService,
    bstock_borrowing::{BorrowingPolicy, BorrowingService},
    bstock_market::{Address, BstockMarketService},
    bstock_trade::{BstockTradeService, SettlementPairs, SettlementTokenMetadata, TradePolicy},
    carry::{CarryPolicy, CarryService},
    liquidity::{LiquidityPolicy, LiquidityService},
    managed_portfolios::ManagedPortfolioService,
    recipes::RecipeService,
    strategy_composer::{StrategyComposerService, StrategyModel},
    wallet_portfolio::WalletPortfolioService,
};

#[tokio::main]
#[expect(
    clippy::too_many_lines,
    reason = "the composition root wires every vertical slice in one place"
)]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stockfront_app=info,tower_http=info".into()),
        )
        .init();
    let Settings {
        binance_web3_api_key,
        binance_web3_api_secret,
        binance_web3_base_url,
        bind_address,
        trade_settlement_tokens,
        max_trade_price_impact_bps,
        trade_slippage_bps,
        trade_settlement_pairs,
        venus_api_base_url,
        bsc_rpc_url,
        venus_comptroller,
        borrow_debt_tokens,
        carry_investment_ids,
        min_carry_spread_bps,
        liquidity_investment_ids,
        liquidity_slippage_bps,
        reserve_api_base_url,
        glider_api_key,
        glider_api_base_url,
        ai,
    } = Settings::from_env()?;
    let binance = BinanceWeb3Client::new(
        &binance_web3_base_url,
        &binance_web3_api_key,
        &binance_web3_api_secret,
        Duration::from_secs(10),
    )?;
    drop((binance_web3_api_key, binance_web3_api_secret));
    let client = Arc::new(binance);
    let market = Arc::new(BstockMarketService::new(client.clone()));
    let wallet_portfolio = Arc::new(WalletPortfolioService::new(client.clone(), client.clone()));
    let trade_policy = TradePolicy::new(
        trade_settlement_tokens.clone(),
        max_trade_price_impact_bps,
        trade_slippage_bps,
        if trade_settlement_pairs {
            SettlementPairs::Allowed
        } else {
            SettlementPairs::Rejected
        },
    )?;
    let trades = Arc::new(BstockTradeService::new(
        client.clone(),
        client.clone(),
        trade_policy.clone(),
    ));
    let venus = Arc::new(
        VenusBorrowingAdapter::new(
            &venus_api_base_url,
            &bsc_rpc_url,
            venus_comptroller,
            client.clone(),
            Duration::from_secs(10),
        )
        .await?,
    );
    let borrowing = Arc::new(BorrowingService::new(
        client.clone(),
        venus.clone(),
        venus.clone(),
        BorrowingPolicy::new(borrow_debt_tokens)?,
    ));
    let liquidity = Arc::new(LiquidityService::new(
        client.clone(),
        client.clone(),
        client.clone(),
        LiquidityPolicy::new(liquidity_investment_ids, liquidity_slippage_bps)?,
    ));
    let carry = Arc::new(CarryService::new(
        borrowing.clone(),
        client.clone(),
        client,
        CarryPolicy::new(carry_investment_ids, min_carry_spread_bps),
    ));
    let reserve = Arc::new(ReserveAdapter::new(
        &reserve_api_base_url,
        Duration::from_secs(10),
        trade_slippage_bps,
    )?);
    let baskets = Arc::new(BasketService::new(reserve.clone(), reserve, trade_policy));
    let glider = Arc::new(GliderAdapter::new(
        &glider_api_base_url,
        glider_api_key,
        Duration::from_secs(10),
    )?);
    let managed_portfolios = Arc::new(ManagedPortfolioService::new(glider));
    let settlement_metadata = venus
        .settlement_token_metadata(&trade_settlement_tokens)
        .await?;
    let recipes = recipe_routes(
        market.clone(),
        trades.clone(),
        borrowing.clone(),
        carry.clone(),
        settlement_metadata,
        venus_comptroller,
        ai,
    )?;
    let app = bstock_router(market)
        .merge(wallet_router(wallet_portfolio))
        .merge(trade_router(trades))
        .merge(borrowing_router(borrowing.clone()))
        .merge(carry_router(carry))
        .merge(liquidity_router(liquidity))
        .merge(basket_router(baskets))
        .merge(managed_portfolio_router(managed_portfolios))
        .merge(recipes);
    serve(bind_address, api(app)).await?;
    Ok(())
}

/// Builds the recipe API and the AI composer that drafts recipes for it.
fn recipe_routes(
    market: Arc<BstockMarketService>,
    trades: Arc<BstockTradeService>,
    borrowing: Arc<BorrowingService>,
    carry: Arc<CarryService>,
    settlement_metadata: Vec<SettlementTokenMetadata>,
    pool: Address,
    ai: AiSettings,
) -> Result<axum::Router, Box<dyn Error>> {
    let model = strategy_model(&ai)?;
    drop(ai);
    let recipes = Arc::new(RecipeService::new(
        market,
        trades,
        borrowing.clone(),
        carry.clone(),
        settlement_metadata.clone(),
        pool,
    ));
    let composer = StrategyComposerService::new(
        recipes.clone(),
        borrowing,
        carry,
        model,
        settlement_metadata,
    );
    Ok(recipe_router(recipes).merge(strategy_router(Arc::new(composer))))
}

/// Builds the configured strategy model; only the selected provider's key is
/// required, so the other may be left unset.
fn strategy_model(ai: &AiSettings) -> Result<Arc<dyn StrategyModel>, Box<dyn Error>> {
    let timeout = Duration::from_secs(ai.request_timeout_seconds);
    Ok(match ai.provider {
        AiProvider::OpenRouter => Arc::new(OpenAiCompatibleClient::new(
            &ai.openrouter_base_url,
            &ai.openrouter_model,
            &ai.openrouter_api_key,
            ai.openrouter_require_parameters,
            timeout,
        )?),
        AiProvider::Gemini => Arc::new(GeminiClient::new(
            &ai.gemini_api_base_url,
            &ai.gemini_model,
            &ai.gemini_api_key,
            timeout,
        )?),
    })
}

async fn serve(bind_address: SocketAddr, app: axum::Router) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        // The server can still shut down when its runtime is dropped.
    }
}
