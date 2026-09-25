mod setting;

use std::{error::Error, sync::Arc, time::Duration};

use setting::Settings;
use stockfront_adapters::{
    binance_web3::BinanceWeb3Client,
    http::{bstock_router, wallet_router},
};
use stockfront_core::bstock_market::BstockMarketService;
use stockfront_core::wallet_portfolio::WalletPortfolioService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Settings {
        binance_web3_api_key,
        binance_web3_api_secret,
        binance_web3_base_url,
        bind_address,
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
    let wallet_portfolio = Arc::new(WalletPortfolioService::new(binance.clone(), binance));
    let app = bstock_router(bstock_market).merge(wallet_router(wallet_portfolio));
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
