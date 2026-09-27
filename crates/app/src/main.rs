mod setting;

use std::{error::Error, sync::Arc, time::Duration};

use setting::Settings;
use stockfront_adapters::{
    binance_web3::BinanceWeb3Client,
    http::{borrowing_router, bstock_router, carry_router, trade_router, wallet_router},
    venus::VenusBorrowingAdapter,
};
use stockfront_core::{
    bstock_borrowing::{BorrowingPolicy, BorrowingService},
    bstock_market::BstockMarketService,
    bstock_trade::{BstockTradeService, TradePolicy},
    carry::{CarryOpportunityService, CarryPolicy},
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
    } = Settings::from_env()?;

    let binance = BinanceWeb3Client::new(
        &binance_web3_base_url,
        &binance_web3_api_key,
        &binance_web3_api_secret,
        Duration::from_secs(10),
    )?;
    drop(binance_web3_api_key);
    drop(binance_web3_api_secret);
    let binance = Arc::new(binance);
    let bstock_market = Arc::new(BstockMarketService::new(binance.clone()));
    let wallet_portfolio = Arc::new(WalletPortfolioService::new(
        binance.clone(),
        binance.clone(),
    ));
    let trade_policy = TradePolicy::new(
        trade_settlement_tokens,
        max_trade_price_impact_bps,
        trade_slippage_bps,
    )?;
    let bstock_trade = Arc::new(BstockTradeService::new(
        binance.clone(),
        binance.clone(),
        trade_policy,
    ));
    let venus = Arc::new(
        VenusBorrowingAdapter::new(
            &venus_api_base_url,
            &bsc_rpc_url,
            venus_comptroller,
            binance.clone(),
            Duration::from_secs(10),
        )
        .await?,
    );
    let borrowing = Arc::new(BorrowingService::new(
        binance.clone(),
        venus.clone(),
        venus,
        BorrowingPolicy::new(borrow_debt_tokens)?,
    ));
    let carry = Arc::new(CarryOpportunityService::new(
        borrowing.clone(),
        binance,
        CarryPolicy::new(carry_investment_ids, min_carry_spread_bps),
    ));
    let app = bstock_router(bstock_market)
        .merge(wallet_router(wallet_portfolio))
        .merge(trade_router(bstock_trade))
        .merge(borrowing_router(borrowing))
        .merge(carry_router(carry));
    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        // The server can still shut down when its runtime is dropped.
    }
}
