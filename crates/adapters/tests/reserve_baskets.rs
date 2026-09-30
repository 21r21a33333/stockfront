use std::{error::Error, time::Duration};

use rust_decimal::Decimal;
use stockfront_adapters::reserve::ReserveBasketCatalog;
use stockfront_core::{
    baskets::{BasketCatalog, BasketKind},
    bstock_market::BNB_CHAIN_ID,
};

#[tokio::test]
#[ignore = "calls the live Reserve API"]
async fn lists_live_active_bnb_index_baskets() -> Result<(), Box<dyn Error>> {
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let catalog = ReserveBasketCatalog::new(&base_url, Duration::from_secs(10))?;

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
