use std::sync::Arc;

use super::{BasketCatalog, BasketProviderError, BasketSummary, ListBasketsQuery};

pub struct BasketService {
    catalog: Arc<dyn BasketCatalog>,
}

impl BasketService {
    #[must_use]
    pub fn new(catalog: Arc<dyn BasketCatalog>) -> Self {
        Self { catalog }
    }

    /// Returns active BNB basket products matching the requested filters.
    ///
    /// # Errors
    ///
    /// Returns [`BasketProviderError`] when the catalog cannot be read.
    pub async fn list(
        &self,
        query: &ListBasketsQuery,
    ) -> Result<Vec<BasketSummary>, BasketProviderError> {
        let mut baskets = self.catalog.list_baskets().await?;
        if let Some(search) = query.search.as_deref() {
            let search = search.to_lowercase();
            baskets.retain(|basket| matches_search(basket, &search));
        }
        if let Some(category) = query.category.as_deref() {
            baskets.retain(|basket| {
                basket
                    .categories
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(category))
            });
        }
        baskets.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.symbol.cmp(&right.symbol))
        });
        baskets.truncate(query.limit);
        Ok(baskets)
    }
}

fn matches_search(basket: &BasketSummary, search: &str) -> bool {
    [basket.name.as_str(), basket.symbol.as_str()]
        .into_iter()
        .chain(basket.categories.iter().map(String::as_str))
        .chain(
            basket
                .assets
                .iter()
                .flat_map(|asset| [asset.name.as_str(), asset.symbol.as_str()]),
        )
        .any(|candidate| candidate.to_lowercase().contains(search))
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use async_trait::async_trait;
    use rust_decimal::Decimal;

    use super::*;
    use crate::baskets::{BasketAssetAllocation, BasketId, BasketKind};

    struct FixedCatalog(Vec<BasketSummary>);

    #[async_trait]
    impl BasketCatalog for FixedCatalog {
        async fn list_baskets(&self) -> Result<Vec<BasketSummary>, BasketProviderError> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn list_filters_search_category_and_limit_deterministically() {
        let service = BasketService::new(Arc::new(FixedCatalog(vec![
            basket(2, "Robotics", "ROBOTS", "AI", "ISRG"),
            basket(1, "AI Infrastructure", "BUILDOUT", "AI", "NVDA"),
            basket(3, "Crypto Index", "CMC20", "Majors", "BTCB"),
        ])));
        let query =
            ListBasketsQuery::new(Some("nvidia".to_owned()), Some("ai".to_owned()), Some(1))
                .unwrap_or_else(|error| panic!("valid query rejected: {error}"));

        let baskets = service
            .list(&query)
            .await
            .unwrap_or_else(|error| panic!("basket list failed: {error}"));

        assert_eq!(baskets.len(), 1);
        assert_eq!(baskets[0].symbol, "BUILDOUT");
    }

    fn basket(
        index: u8,
        name: &str,
        symbol: &str,
        category: &str,
        asset_symbol: &str,
    ) -> BasketSummary {
        BasketSummary {
            id: BasketId::bnb(Address::with_last_byte(index)),
            kind: BasketKind::IndexToken,
            provider_id: "reserve".to_owned(),
            provider_name: "Reserve".to_owned(),
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            price_usd: None,
            market_cap_usd: None,
            annual_fee_percent: Some(Decimal::new(3, 1)),
            assets: vec![BasketAssetAllocation {
                contract: Address::with_last_byte(index.saturating_add(10)),
                symbol: asset_symbol.to_owned(),
                name: if asset_symbol == "NVDA" {
                    "Nvidia".to_owned()
                } else {
                    asset_symbol.to_owned()
                },
                weight_percent: Decimal::ONE,
            }],
            categories: vec![category.to_owned()],
            description: None,
            image_url: None,
        }
    }
}
