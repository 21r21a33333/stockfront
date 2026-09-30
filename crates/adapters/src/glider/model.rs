use std::str::FromStr;

use alloy_primitives::Address;
use glider_api::models::{DiscoveryStrategyItem, StrategyAllocationAsset};
use rust_decimal::Decimal;
use stockfront_core::{
    baskets::BasketKind,
    bstock_market::BNB_CHAIN_ID,
    managed_portfolios::{
        ManagedPortfolioAllocation, ManagedPortfolioPerformance, ManagedPortfolioProviderError,
        ManagedPortfolioTemplate, ManagedPortfolioTemplateMetrics,
    },
};

const BNB_ERC20_PREFIX: &str = "eip155:56/erc20:";

pub(super) struct BnbManagedPortfolioTemplate(ManagedPortfolioTemplate);

struct BnbManagedPortfolioAllocation(ManagedPortfolioAllocation);

impl From<BnbManagedPortfolioTemplate> for ManagedPortfolioTemplate {
    fn from(value: BnbManagedPortfolioTemplate) -> Self {
        value.0
    }
}

impl From<BnbManagedPortfolioAllocation> for ManagedPortfolioAllocation {
    fn from(value: BnbManagedPortfolioAllocation) -> Self {
        value.0
    }
}

impl TryFrom<DiscoveryStrategyItem> for BnbManagedPortfolioTemplate {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: DiscoveryStrategyItem) -> Result<Self, Self::Error> {
        let name = non_empty(&value.name).ok_or(ManagedPortfolioProviderError::InvalidData)?;
        let allocation = value
            .allocation
            .assets
            .into_iter()
            .map(BnbManagedPortfolioAllocation::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let allocation = allocation
            .into_iter()
            .map(ManagedPortfolioAllocation::from)
            .collect();
        let metrics = *value.metrics;
        let performance = metrics
            .performance
            .summary
            .windows
            .into_iter()
            .filter_map(|window| {
                Decimal::from_str(&window.percent_change)
                    .ok()
                    .map(|percent_change| ManagedPortfolioPerformance {
                        window: window.window,
                        percent_change,
                        since: window.since,
                    })
            })
            .collect();

        Ok(Self(ManagedPortfolioTemplate {
            id: value.strategy_id,
            kind: BasketKind::ManagedPortfolio,
            provider_id: "glider".to_owned(),
            provider_name: "Glider".to_owned(),
            name,
            description: value.description.as_deref().and_then(non_empty),
            advertised_max_apy_percent: value.max_apy.and_then(|apy| Decimal::from_str(&apy).ok()),
            allocation,
            metrics: ManagedPortfolioTemplateMetrics {
                tvl_usd: metrics.tvl_usd.and_then(|tvl| Decimal::from_str(&tvl).ok()),
                portfolio_count: metrics
                    .portfolio_count
                    .and_then(|count| u64::try_from(count).ok()),
                performance,
            },
        }))
    }
}

impl TryFrom<StrategyAllocationAsset> for BnbManagedPortfolioAllocation {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: StrategyAllocationAsset) -> Result<Self, Self::Error> {
        let contract = value
            .asset_id
            .strip_prefix(BNB_ERC20_PREFIX)
            .ok_or(ManagedPortfolioProviderError::InvalidData)?
            .parse::<Address>()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        let weight_percent = Decimal::from_str(&value.weight)
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;

        Ok(Self(ManagedPortfolioAllocation {
            chain_id: BNB_CHAIN_ID,
            contract,
            weight_percent,
        }))
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bnb_strategy_and_rejects_other_chains() {
        let strategy = serde_json::from_value::<DiscoveryStrategyItem>(serde_json::json!({
            "strategyId": "01K6D5JCKM1N7Z8D2YQ0Q3F1AB",
            "name": "Technology Leaders",
            "description": "  BNB stocks  ",
            "maxApy": "10.25",
            "allocation": {
                "assets": [{
                    "assetId": "eip155:56/erc20:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
                    "weight": "100"
                }]
            },
            "createdAt": "2026-09-30T00:00:00.000Z",
            "metrics": {
                "tvlUsd": "1200.50",
                "portfolioCount": 4,
                "performance": {
                    "summary": {
                        "windows": [{
                            "window": "1m",
                            "percentChange": "2.75",
                            "since": "2026-08-30"
                        }]
                    }
                }
            },
            "canMirror": true
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        let template = ManagedPortfolioTemplate::from(
            BnbManagedPortfolioTemplate::try_from(strategy)
                .unwrap_or_else(|error| panic!("valid BNB strategy rejected: {error}")),
        );

        assert_eq!(template.kind, BasketKind::ManagedPortfolio);
        assert_eq!(template.description.as_deref(), Some("BNB stocks"));
        assert_eq!(
            template.advertised_max_apy_percent,
            Some(Decimal::new(1_025, 2))
        );
        assert_eq!(template.allocation.len(), 1);
        assert_eq!(template.allocation[0].chain_id, BNB_CHAIN_ID);
        assert_eq!(template.metrics.tvl_usd, Some(Decimal::new(120_050, 2)));
        assert_eq!(template.metrics.portfolio_count, Some(4));
        assert_eq!(template.metrics.performance[0].window, "1m");

        let base_strategy = serde_json::from_value::<DiscoveryStrategyItem>(serde_json::json!({
            "strategyId": "01K6D5JCKM1N7Z8D2YQ0Q3F1AC",
            "name": "Base strategy",
            "description": null,
            "allocation": {
                "assets": [{
                    "assetId": "eip155:8453/erc20:0x0000000000000000000000000000000000000001",
                    "weight": "100"
                }]
            },
            "createdAt": null,
            "metrics": {
                "tvlUsd": null,
                "portfolioCount": null,
                "performance": { "summary": { "windows": [] } }
            },
            "canMirror": true
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        assert!(BnbManagedPortfolioTemplate::try_from(base_strategy).is_err());
    }
}
