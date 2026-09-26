use std::{env, error::Error, io, sync::Arc, time::Duration};

use alloy_primitives::address;
use stockfront_adapters::{binance_web3::BinanceWeb3Client, venus::VenusBorrowingReader};
use stockfront_core::{
    bstock_borrowing::BorrowingReader,
    bstock_market::{BNB_CHAIN_ID, BstockId},
    wallet_portfolio::WalletAddress,
};

const BINANCE_WEB3_URL: &str = "https://web3.binance.com/build";
const VENUS_API_URL: &str = "https://api.venus.io";
const VENUS_COMPTROLLER: alloy_primitives::Address =
    address!("fD36E2c2a6789Db23113685031d7F16329158384");
const TSLAB: alloy_primitives::Address = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and live provider access"]
async fn reads_bstock_market_and_empty_pooled_account() -> Result<(), Box<dyn Error>> {
    let binance = Arc::new(BinanceWeb3Client::new(
        BINANCE_WEB3_URL,
        &env::var("BINANCE_WEB3_API_KEY")?,
        &env::var("BINANCE_WEB3_API_SECRET")?,
        Duration::from_secs(15),
    )?);
    let reader = VenusBorrowingReader::new(
        VENUS_API_URL,
        VENUS_COMPTROLLER,
        binance,
        Duration::from_secs(15),
    )?;
    let markets = reader
        .read_markets(
            &[BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: TSLAB,
            }],
            &[USDT],
        )
        .await
        .map_err(|error| io::Error::other(format!("market read failed: {error}")))?;
    assert_eq!(markets.len(), 1);
    assert_eq!(markets[0].collateral.contract, TSLAB);
    assert_eq!(markets[0].debt.contract, USDT);
    assert!(markets[0].observed_at_ms > 0);

    let wallet = WalletAddress::try_from(address!("0000000000000000000000000000000000000001"))?;
    let account = reader
        .read_account(wallet)
        .await
        .map_err(|error| io::Error::other(format!("account read failed: {error}")))?;
    assert!(account.positions.is_empty());
    assert!(account.health_factor.is_none());
    assert_eq!(account.chain_id, BNB_CHAIN_ID);
    assert!(account.observed_at_ms > 0);
    Ok(())
}
