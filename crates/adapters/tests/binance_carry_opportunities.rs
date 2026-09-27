mod support;

use std::{collections::HashSet, error::Error};

use alloy_primitives::address;
use rust_decimal::Decimal;
use stockfront_core::carry::EarnOpportunityReader;

use support::live_client;

const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn discovers_live_usdt_earn_opportunities() -> Result<(), Box<dyn Error>> {
    let opportunities = live_client()?.opportunities(USDT).await?;

    assert!(
        !opportunities.is_empty(),
        "Binance returned no USDT earn opportunities"
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
            && opportunity.asset.contract == USDT
            && !opportunity.asset.symbol.is_empty()
            && !opportunity.base_rate_display.is_empty()
            && opportunity
                .reported_tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
            && opportunity.observed_at_ms > 0
    }));

    Ok(())
}
