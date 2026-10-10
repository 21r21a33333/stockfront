use std::{sync::Arc, time::Duration};

use alloy_primitives::Address;
use binance_web3_rwa::apis::configuration::Configuration;
use reqwest_middleware::ClientBuilder;
use stockfront_core::bstock_market::BstockSummary;

use super::{
    auth::{BinanceConfigurationError, BinanceWeb3Authentication},
    defi::ProviderInvestmentDetail,
};
use crate::cache::TtlCache;

/// How long a cached catalogue listing is served before it is read again.
///
/// The listing carries live prices alongside stable token identities, so the
/// window is short enough that a builder never shows a visibly stale quote.
const CATALOG_TTL: Duration = Duration::from_mins(1);
/// `DeFi` product listings change when a protocol is added, not minute to minute.
const INVESTMENT_LIST_TTL: Duration = Duration::from_mins(1);
/// A product's terms move slowly; only its APY drifts, and only by basis points.
const INVESTMENT_DETAIL_TTL: Duration = Duration::from_mins(5);

/// Which product listing an investment-id cache entry belongs to.
///
/// Earn and liquidity ask the same endpoint about the same asset but mean
/// different things by it, so they cannot share a cache entry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum InvestmentListKind {
    Earn,
    Liquidity,
}

pub struct BinanceWeb3Client {
    pub(super) configuration: Configuration,
    pub(super) bstocks: TtlCache<(), Arc<Vec<BstockSummary>>>,
    pub(super) investment_lists: TtlCache<(InvestmentListKind, Address), Arc<Vec<String>>>,
    pub(super) investment_details: TtlCache<String, Option<Arc<ProviderInvestmentDetail>>>,
}

impl BinanceWeb3Client {
    /// Creates an authenticated Binance Web3 client.
    ///
    /// # Errors
    ///
    /// Returns [`BinanceConfigurationError`] when credentials or the HTTP
    /// client cannot be configured.
    pub fn new(
        base_url: &str,
        api_key: &str,
        api_secret: &str,
        timeout: Duration,
    ) -> Result<Self, BinanceConfigurationError> {
        let authentication = BinanceWeb3Authentication::new(api_key, api_secret)?;
        let client = reqwest::Client::builder()
            .gzip(true)
            .timeout(timeout)
            .build()?;
        let client = ClientBuilder::new(client).with(authentication).build();
        let configuration = Configuration {
            base_path: base_url.trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client,
            ..Configuration::default()
        };
        Ok(Self {
            configuration,
            bstocks: TtlCache::new(1, CATALOG_TTL),
            investment_lists: TtlCache::new(64, INVESTMENT_LIST_TTL),
            investment_details: TtlCache::new(512, INVESTMENT_DETAIL_TTL),
        })
    }
}
