use std::{error::Error, time::Duration};

use rust_decimal::Decimal;
use stockfront_adapters::reserve::ReserveAdapter;
use stockfront_core::{
    baskets::{BasketKind, BasketReader},
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

const DEFAULT_WALLET: alloy_primitives::Address =
    alloy_primitives::address!("4b393bec5dee11799f698440ab19c0fbfe15d74c");

#[tokio::test]
#[ignore = "calls the live Reserve API"]
async fn lists_live_active_bnb_index_baskets() -> Result<(), Box<dyn Error>> {
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let catalog = ReserveAdapter::new(&base_url, Duration::from_secs(10), 50)?;

    let baskets = catalog.list_baskets().await?;

    assert!(
        !baskets.is_empty(),
        "Reserve returned no active BNB baskets"
    );
    assert!(
        baskets
            .iter()
            .any(|basket| basket.categories.iter().any(|tag| tag == "RWA")),
        "Reserve returned no BNB RWA basket"
    );
    for basket in baskets {
        assert_eq!(basket.id.chain_id, BNB_CHAIN_ID);
        assert_ne!(basket.id.contract, alloy_primitives::Address::ZERO);
        assert_eq!(basket.kind, BasketKind::IndexToken);
        assert_eq!(basket.provider_id, "reserve");
        assert_eq!(basket.provider_name, "Reserve");
        assert!(!basket.name.is_empty());
        assert!(!basket.symbol.is_empty());
        assert!(!basket.assets.is_empty());
        if let Some(price) = basket.price_usd {
            assert!(price > Decimal::ZERO);
        }
        if let Some(market_cap) = basket.market_cap_usd {
            assert!(market_cap >= Decimal::ZERO);
        }
        if let Some(fee) = basket.annual_fee_percent {
            assert!(fee >= Decimal::ZERO);
        }
        for asset in basket.assets {
            assert_ne!(asset.contract, alloy_primitives::Address::ZERO);
            assert!(!asset.name.is_empty());
            assert!(!asset.symbol.is_empty());
            assert!(asset.weight_percent > Decimal::ZERO);
        }
    }
    Ok(())
}

#[tokio::test]
#[ignore = "calls the live Reserve API"]
async fn reads_live_bnb_basket_positions() -> Result<(), Box<dyn Error>> {
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let wallet = std::env::var("STOCKFRONT_TEST_WALLET")
        .ok()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(DEFAULT_WALLET);
    let reader = ReserveAdapter::new(&base_url, Duration::from_secs(10), 50)?;

    let account = reader.positions(WalletAddress::try_from(wallet)?).await?;

    assert_eq!(account.chain_id, BNB_CHAIN_ID);
    assert!(account.observed_at_ms > 0);
    let expected_total = account
        .positions
        .iter()
        .map(|position| position.value_usd)
        .sum::<Decimal>();
    assert_eq!(account.total_value_usd, expected_total);
    for position in account.positions {
        assert_eq!(position.id.chain_id, BNB_CHAIN_ID);
        assert_ne!(position.id.contract, alloy_primitives::Address::ZERO);
        assert_eq!(position.kind, BasketKind::IndexToken);
        assert_eq!(position.provider_id, "reserve");
        assert_eq!(position.provider_name, "Reserve");
        assert!(!position.name.is_empty());
        assert!(!position.symbol.is_empty());
        assert!(position.token_amount > Decimal::ZERO);
        assert!(position.value_usd >= Decimal::ZERO);
    }
    Ok(())
}
