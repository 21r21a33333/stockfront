mod support;
#[path = "support/wallet.rs"]
mod wallet_support;

use std::{error::Error, sync::Arc};

use alloy_primitives::U256;
use stockfront_core::{bstock_market::BNB_CHAIN_ID, wallet_portfolio::WalletPortfolioService};

use support::live_client;
use wallet_support::binance_hot_wallet;

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn reads_live_bstock_portfolio_from_binance() -> Result<(), Box<dyn Error>> {
    let wallet = binance_hot_wallet()?;
    let client = Arc::new(live_client()?);
    let service = WalletPortfolioService::new(client.clone(), client);

    let portfolio = service.portfolio(wallet).await?;

    assert_eq!(portfolio.wallet, wallet);
    assert_eq!(portfolio.chain_id, BNB_CHAIN_ID);
    assert!(portfolio.observed_at_ms > 0);
    assert!(!portfolio.positions.is_empty());
    assert!(
        portfolio
            .positions
            .iter()
            .all(|position| position.raw_amount > U256::ZERO)
    );

    Ok(())
}
