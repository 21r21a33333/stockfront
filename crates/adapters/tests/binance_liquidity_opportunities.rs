mod support;

use std::{collections::HashSet, error::Error, sync::Arc};

use alloy_primitives::{Address, address};
use rust_decimal::Decimal;
use stockfront_core::liquidity::LiquidityService;

use support::live_client;

const NVDAB: Address = address!("02Fca66C1D1aFB4E2A7884261eB00F63598a7436");

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn discovers_live_nvidia_bstock_liquidity_opportunities() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(client.clone(), client);
    let opportunities = service.opportunities(NVDAB).await?;

    assert!(
        !opportunities.is_empty(),
        "Binance returned no NVIDIA bStock liquidity opportunities"
    );
    let unique_ids = opportunities
        .iter()
        .map(|opportunity| opportunity.investment_id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(unique_ids.len(), opportunities.len());
    assert!(opportunities.iter().all(|opportunity| {
        !opportunity.investment_id.is_empty()
            && !opportunity.investment_name.is_empty()
            && !opportunity.protocol_id.is_empty()
            && !opportunity.protocol_name.is_empty()
            && opportunity
                .protocol_logo_url
                .as_ref()
                .is_none_or(|url| !url.is_empty())
            && opportunity
                .pool_address
                .is_none_or(|address| address != Address::ZERO)
            && opportunity.assets.len() == 2
            && opportunity
                .assets
                .iter()
                .all(|asset| asset.contract != Address::ZERO && !asset.symbol.is_empty())
            && opportunity
                .assets
                .iter()
                .any(|asset| asset.contract == NVDAB)
            && !opportunity.reported_rate_display.is_empty()
            && opportunity
                .reported_tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
            && opportunity.fee_rate.is_none_or(|fee| fee >= Decimal::ZERO)
            && opportunity.observed_at_ms > 0
    }));

    Ok(())
}
