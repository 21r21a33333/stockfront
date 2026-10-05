use std::sync::Arc;

pub use alloy_primitives::Address;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::serialization::serialize_as_string;

pub const BNB_CHAIN_ID: u64 = 56;
pub const DEFAULT_LIST_LIMIT: usize = 50;
pub const MAX_LIST_LIMIT: usize = 100;

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BstockId {
    pub chain_id: u64,
    #[serde(serialize_with = "serialize_as_string")]
    #[schema(value_type = String, format = "evm-address")]
    pub contract: Address,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Stock,
    PreIpo,
    Etf,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketSession {
    PreMarket,
    Regular,
    PostMarket,
    Overnight,
    Closed,
    Paused,
    Unknown(String),
}

impl Serialize for MarketSession {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let session = match self {
            Self::PreMarket => "premarket",
            Self::Regular => "regular",
            Self::PostMarket => "postmarket",
            Self::Overnight => "overnight",
            Self::Closed => "closed",
            Self::Paused => "paused",
            Self::Unknown(session) => session,
        };
        serializer.serialize_str(session)
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, nullable = false)]
    pub session: Option<MarketSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub tradable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub next_open_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub next_close_at_ms: Option<i64>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BstockSummary {
    #[serde(flatten)]
    pub id: BstockId,
    pub token_name: String,
    pub token_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub token_logo_url: Option<String>,
    pub token_decimals: u8,
    pub underlying_ticker: String,
    pub underlying_name: String,
    pub asset_kind: AssetKind,
    pub token_to_share_ratio: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub token_price_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reference_price_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub volume_24h_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub market_cap_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub market_status: Option<MarketStatus>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ceo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    pub concepts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Protection {
    pub kind: String,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_52w_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_52w_usd: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_24h_shares: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_daily_volume_1y: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_shares: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnover_rate_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pe_ratio_ttm: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pb_ratio: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividend_yield_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_dividend_usd: Option<Decimal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BstockDetails {
    pub summary: BstockSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<CompanyProfile>,
    pub protections: Vec<Protection>,
    pub market_metrics: MarketMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_updated_at_ms: Option<i64>,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum CatalogError {
    #[error("the market data provider is unavailable")]
    Unavailable,
    #[error("the market data provider returned invalid data for {field}")]
    InvalidData { field: &'static str },
}

#[async_trait]
pub trait BstockCatalog: Send + Sync {
    async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError>;

    async fn find_bstock(&self, id: BstockId) -> Result<Option<BstockDetails>, CatalogError>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct ListBstocksQuery {
    pub search: Option<String>,
    pub limit: usize,
}

impl ListBstocksQuery {
    /// Builds a bounded catalog query.
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::InvalidLimit`] when the requested limit is zero or
    /// exceeds [`MAX_LIST_LIMIT`].
    pub fn new(search: Option<String>, limit: Option<usize>) -> Result<Self, QueryError> {
        let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT);
        if limit == 0 || limit > MAX_LIST_LIMIT {
            return Err(QueryError::InvalidLimit);
        }

        let search = search
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Ok(Self { search, limit })
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum QueryError {
    #[error("limit must be between 1 and {MAX_LIST_LIMIT}")]
    InvalidLimit,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum BstockMarketError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error("bStocks are only supported on BNB Chain ({BNB_CHAIN_ID})")]
    UnsupportedChain,
}

pub struct BstockMarketService {
    catalog: Arc<dyn BstockCatalog>,
}

impl BstockMarketService {
    pub fn new(catalog: Arc<dyn BstockCatalog>) -> Self {
        Self { catalog }
    }

    /// Returns the bStocks matching a catalog query.
    ///
    /// # Errors
    ///
    /// Returns [`BstockMarketError::Catalog`] when the catalog cannot provide
    /// valid market data.
    pub async fn list(
        &self,
        query: &ListBstocksQuery,
    ) -> Result<Vec<BstockSummary>, BstockMarketError> {
        let mut assets = self.catalog.list_bstocks().await?;
        if let Some(search) = query.search.as_ref() {
            let needle = search.to_lowercase();
            assets.retain(|asset| matches_search(asset, &needle));
        }
        assets.sort_by(|left, right| {
            left.underlying_ticker
                .cmp(&right.underlying_ticker)
                .then_with(|| left.token_symbol.cmp(&right.token_symbol))
        });
        assets.truncate(query.limit);
        Ok(assets)
    }

    /// Returns the current details for one bStock identity.
    ///
    /// # Errors
    ///
    /// Returns [`BstockMarketError::UnsupportedChain`] outside BNB Chain, or
    /// [`BstockMarketError::Catalog`] when market data cannot be obtained.
    pub async fn details(&self, id: BstockId) -> Result<Option<BstockDetails>, BstockMarketError> {
        if id.chain_id != BNB_CHAIN_ID {
            return Err(BstockMarketError::UnsupportedChain);
        }
        self.catalog.find_bstock(id).await.map_err(Into::into)
    }
}

fn matches_search(asset: &BstockSummary, needle: &str) -> bool {
    [
        asset.token_name.as_str(),
        asset.token_symbol.as_str(),
        asset.underlying_ticker.as_str(),
        asset.underlying_name.as_str(),
    ]
    .iter()
    .any(|candidate| candidate.to_lowercase().contains(needle))
        || asset
            .id
            .contract
            .to_string()
            .to_lowercase()
            .contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedCatalog(Vec<BstockSummary>);

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(self.0.clone())
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn list_searches_stable_identity_fields_before_applying_limit() {
        let catalog = FixedCatalog(vec![summary("MSFT", "Microsoft"), summary("AAPL", "Apple")]);
        let service = BstockMarketService::new(Arc::new(catalog));
        let query = ListBstocksQuery::new(Some("app".to_owned()), Some(1))
            .unwrap_or_else(|error| panic!("valid query rejected: {error}"));

        let assets = service
            .list(&query)
            .await
            .unwrap_or_else(|error| panic!("catalog failed: {error}"));

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].underlying_ticker, "AAPL");
    }

    #[tokio::test]
    async fn details_rejects_non_bnb_identity_before_calling_the_catalog() {
        let service = BstockMarketService::new(Arc::new(FixedCatalog(Vec::new())));
        let id = BstockId {
            chain_id: 1,
            contract: address(),
        };

        assert_eq!(
            service.details(id).await,
            Err(BstockMarketError::UnsupportedChain)
        );
    }

    fn summary(ticker: &str, name: &str) -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: address(),
            },
            token_name: format!("{name} token"),
            token_symbol: format!("{ticker}x"),
            token_logo_url: None,
            token_decimals: 18,
            underlying_ticker: ticker.to_owned(),
            underlying_name: name.to_owned(),
            asset_kind: AssetKind::Stock,
            token_to_share_ratio: Decimal::ONE,
            token_price_usd: None,
            reference_price_usd: None,
            volume_24h_usd: None,
            market_cap_usd: None,
            market_status: None,
            tags: Vec::new(),
        }
    }

    fn address() -> Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap_or_else(|error| panic!("test address is invalid: {error}"))
    }
}
