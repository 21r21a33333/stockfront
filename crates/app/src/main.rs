mod setting;

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use setting::Settings;
use stockfront_adapters::{
    binance_web3::BinanceWeb3Client,
    glider::GliderAdapter,
    http::{
        basket_router, borrowing_router, bstock_router, carry_router, liquidity_router,
        managed_portfolio_router, recipe_router, trade_router, wallet_router,
    },
    reserve::ReserveAdapter,
    venus::VenusBorrowingAdapter,
};
use stockfront_core::{
    baskets::BasketService,
    bstock_borrowing::{BorrowingPolicy, BorrowingService},
    bstock_market::BstockMarketService,
    bstock_trade::{BstockTradeService, TradePolicy},
    carry::{CarryPolicy, CarryService},
    liquidity::{LiquidityPolicy, LiquidityService},
    managed_portfolios::ManagedPortfolioService,
    recipes::RecipeService,
    wallet_portfolio::WalletPortfolioService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Settings {
        binance_web3_api_key,
        binance_web3_api_secret,
        binance_web3_base_url,
        bind_address,
        trade_settlement_tokens,
        max_trade_price_impact_bps,
        trade_slippage_bps,
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
        venus,
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
    let recipes = RecipeService::new(
        market.clone(),
        trades.clone(),
        borrowing.clone(),
        trade_settlement_tokens,
        venus_comptroller,
    );
    let app = bstock_router(market)
        .merge(wallet_router(wallet_portfolio))
        .merge(trade_router(trades))
        .merge(borrowing_router(borrowing))
        .merge(carry_router(carry))
        .merge(liquidity_router(liquidity))
        .merge(basket_router(baskets))
        .merge(managed_portfolio_router(managed_portfolios))
        .merge(recipe_router(Arc::new(recipes)));
    serve(bind_address, app).await?;
    Ok(())
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
