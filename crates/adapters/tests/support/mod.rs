use std::{env, error::Error, time::Duration};

use stockfront_adapters::binance_web3::BinanceWeb3Client;

const DEFAULT_BINANCE_WEB3_BASE_URL: &str = "https://web3.binance.com/build";

pub fn live_client() -> Result<BinanceWeb3Client, Box<dyn Error>> {
    let api_key = env::var("BINANCE_WEB3_API_KEY")?;
    let api_secret = env::var("BINANCE_WEB3_API_SECRET")?;
    let base_url = env::var("BINANCE_WEB3_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_BINANCE_WEB3_BASE_URL.to_owned());

    Ok(BinanceWeb3Client::new(
        &base_url,
        &api_key,
        &api_secret,
        Duration::from_secs(10),
    )?)
}
