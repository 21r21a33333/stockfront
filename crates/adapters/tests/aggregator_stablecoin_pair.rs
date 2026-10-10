//! Does the venue quote a pair with no bStock in it?
//!
//! Stockfront's trade policy requires a bStock on one side, but the provider
//! call behind it is a generic aggregated-swap quote. Whether a settlement pair
//! could ever be offered depends on the venue, not on that policy, so this asks
//! the venue directly before any policy is loosened.

mod support;

use std::error::Error;

use alloy_primitives::{Address, address};
use stockfront_core::{
    bstock_trade::{TradeGateway, TradeQuoteRequest},
    wallet_portfolio::WalletAddress,
};

use support::live_client;

const USDT: Address = address!("55d398326f99059ff775485246999027b3197955");
const LISUSD: Address = address!("0782b6d8c4551b9760e74c0545a9bcd90bdc41e5");
const WALLET: Address = address!("4b393bec5dee11799f698440ab19c0fbfe15d74c");

#[tokio::test]
#[ignore = "asks the live Binance aggregator for a quote"]
async fn aggregator_quotes_a_settlement_only_pair() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let quotes = client
        .quotes(&TradeQuoteRequest {
            wallet: WalletAddress::try_from(WALLET)?,
            sell_token: LISUSD,
            buy_token: USDT,
            sell_amount: alloy_primitives::U256::from(9_000_000_000_000_000_000_u128),
        })
        .await?;

    for quote in &quotes {
        println!(
            "venue={} mode={:?} buy={} impact={}",
            quote.venue,
            quote.execution_mode,
            quote.buy.amount,
            quote.price_impact_percent.unwrap_or_default()
        );
    }
    assert!(
        !quotes.is_empty(),
        "the venue offered no settlement-pair route"
    );
    Ok(())
}
