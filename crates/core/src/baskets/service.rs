use std::sync::Arc;

use super::{BasketCatalog, BasketProviderError, BasketSummary, ListBasketsQuery};
use crate::bstock_trade::{
    ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitOrderRequest,
    SubmittedOrder, TradeCatalogError, TradeError, TradeGateway, TradeOrderStatus, TradePolicy,
    TradeQuote, TradeQuoteRequest, select_quotes,
};

pub struct BasketService {
    catalog: Arc<dyn BasketCatalog>,
    gateway: Arc<dyn TradeGateway>,
    policy: TradePolicy,
}

impl BasketService {
    #[must_use]
    pub fn new(
        catalog: Arc<dyn BasketCatalog>,
        gateway: Arc<dyn TradeGateway>,
        policy: TradePolicy,
    ) -> Self {
        Self {
            catalog,
            gateway,
            policy,
        }
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
        Ok(filter_baskets(self.catalog.list_baskets().await?, query))
    }

    /// Returns executable quotes for a supported BNB basket.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the pair is unsupported or the provider cannot quote it.
    pub async fn quotes(&self, request: &TradeQuoteRequest) -> Result<Vec<TradeQuote>, TradeError> {
        self.validate_trade(request).await?;
        select_quotes(self.gateway.quotes(request).await?, &self.policy)
    }

    /// Returns the exact approval transaction for a basket trade.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the amount is zero or the provider rejects the request.
    pub async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeError> {
        if request.amount.is_zero() {
            return Err(TradeError::InvalidAmount);
        }
        self.gateway
            .prepare_approval(request)
            .await
            .map_err(Into::into)
    }

    /// Returns the fresh EIP-712 payload for a basket trade.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the pair is unsupported or the provider rejects the quote.
    pub async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
    ) -> Result<PreparedTrade, TradeError> {
        self.validate_trade(&request.trade).await?;
        self.gateway
            .prepare_trade(request, self.policy.slippage_percent())
            .await
            .map_err(Into::into)
    }

    /// Relays a signed basket RFQ order.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the provider rejects the order.
    pub async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, TradeError> {
        self.gateway.submit_order(request).await.map_err(Into::into)
    }

    /// Returns the current basket RFQ order state.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the provider is unavailable or the order is unknown.
    pub async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, TradeError> {
        self.gateway
            .order_status(order_id)
            .await
            .map_err(Into::into)
    }

    async fn validate_trade(&self, request: &TradeQuoteRequest) -> Result<(), TradeError> {
        if request.sell_amount.is_zero() {
            return Err(TradeError::InvalidAmount);
        }
        let contract = self
            .policy
            .asset_for_pair(request.sell_token, request.buy_token)
            .ok_or(TradeError::UnsupportedTradePair)?;
        let supported = self
            .catalog
            .list_baskets()
            .await
            .map_err(map_basket_catalog_error)?
            .iter()
            .any(|basket| basket.id.contract == contract);
        if supported {
            Ok(())
        } else {
            Err(TradeError::UnsupportedTradePair)
        }
    }
}

fn map_basket_catalog_error(error: BasketProviderError) -> TradeCatalogError {
    match error {
        BasketProviderError::Unavailable => TradeCatalogError::Unavailable,
        BasketProviderError::InvalidData => TradeCatalogError::InvalidData,
    }
}

fn filter_baskets(mut baskets: Vec<BasketSummary>, query: &ListBasketsQuery) -> Vec<BasketSummary> {
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
    baskets
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
    use rust_decimal::Decimal;

    use super::*;
    use crate::baskets::{BasketAssetAllocation, BasketId, BasketKind};

    #[test]
    fn list_filters_search_category_and_limit_deterministically() {
        let baskets = filter_baskets(
            vec![
                basket(2, "Robotics", "ROBOTS", "AI", "ISRG"),
                basket(1, "AI Infrastructure", "BUILDOUT", "AI", "NVDA"),
                basket(3, "Crypto Index", "CMC20", "Majors", "BTCB"),
            ],
            &ListBasketsQuery::new(Some("nvidia".to_owned()), Some("ai".to_owned()), Some(1))
                .unwrap_or_else(|error| panic!("valid query rejected: {error}")),
        );

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
