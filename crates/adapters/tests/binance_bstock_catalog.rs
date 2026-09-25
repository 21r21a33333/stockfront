mod support;

use std::{error::Error, io};

use alloy_primitives::Address;
use stockfront_core::bstock_market::{BNB_CHAIN_ID, BstockCatalog};

use support::live_client;

const NVIDIA_TICKER: &str = "NVDA";

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn lists_live_bstocks_from_binance() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let assets = client.list_bstocks().await?;

    assert!(
        !assets.is_empty(),
        "Binance returned an empty bStock catalog"
    );
    assert!(
        assets.iter().all(|asset| {
            asset.id.chain_id == BNB_CHAIN_ID
                && asset.id.contract != Address::ZERO
                && !asset.token_symbol.is_empty()
                && !asset.underlying_ticker.is_empty()
        }),
        "Binance returned an unusable bStock identity"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn fetches_live_bstock_details_from_binance() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let assets = client.list_bstocks().await?;
    let asset = assets
        .iter()
        .find(|asset| asset.underlying_ticker == NVIDIA_TICKER)
        .ok_or_else(|| io::Error::other("Binance did not return the NVIDIA bStock"))?;
    let details = client
        .find_bstock(asset.id)
        .await?
        .ok_or_else(|| io::Error::other("catalogued bStock details were not found"))?;

    assert_eq!(details.summary.id, asset.id);
    assert_eq!(details.summary.token_symbol, asset.token_symbol);
    assert_eq!(details.summary.underlying_ticker, asset.underlying_ticker);
    assert!(
        details
            .company
            .as_ref()
            .and_then(|company| company.description.as_deref())
            .is_some_and(|description| !description.is_empty()),
        "Binance returned NVIDIA without its company description"
    );

    Ok(())
}
