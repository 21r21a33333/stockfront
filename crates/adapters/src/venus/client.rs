use std::{sync::Arc, time::Duration};

use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy_primitives::Address;
use reqwest_middleware::ClientBuilder;
use stockfront_core::bstock_market::BNB_CHAIN_ID;
use thiserror::Error;
use url::Url;
use venus_api::{apis::configuration::Configuration, models::Market};

use crate::{binance_web3::BinanceWeb3Client, cache::TtlCache};

/// How long one read of the Venus market listing is reused.
///
/// Listing the markets costs seconds and every borrowing route needs it, often
/// twice. Rates and caps drift per block, so the window stays short enough that
/// a preview and the transaction it produces agree.
const MARKETS_TTL: Duration = Duration::from_secs(30);

pub struct VenusBorrowingAdapter {
    pub(super) api: Configuration,
    pub(super) comptroller: Address,
    pub(super) binance: Arc<BinanceWeb3Client>,
    pub(super) provider: DynProvider,
    pub(super) markets: TtlCache<(), Arc<Vec<Market>>>,
}

impl VenusBorrowingAdapter {
    /// Creates the combined Venus read and transaction adapter.
    ///
    /// # Errors
    ///
    /// Returns an error when the Venus API or BNB Chain provider cannot be configured.
    pub async fn new(
        api_base_url: &str,
        rpc_url: &str,
        comptroller: Address,
        binance: Arc<BinanceWeb3Client>,
        timeout: Duration,
    ) -> Result<Self, VenusConfigurationError> {
        let client = reqwest::Client::builder()
            .gzip(true)
            .timeout(timeout)
            .build()
            .map_err(|_| VenusConfigurationError::Unavailable)?;
        let api = Configuration {
            base_path: api_base_url.trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client: ClientBuilder::new(client).build(),
            ..Configuration::default()
        };
        let provider = ProviderBuilder::new()
            .connect_http(Url::parse(rpc_url)?)
            .erased();
        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|_| VenusConfigurationError::Unavailable)?;
        if chain_id != BNB_CHAIN_ID {
            return Err(VenusConfigurationError::UnexpectedChain { chain_id });
        }

        Ok(Self {
            api,
            comptroller,
            binance,
            provider,
            markets: TtlCache::new(1, MARKETS_TTL),
        })
    }
}

#[derive(Debug, Error)]
pub enum VenusConfigurationError {
    #[error("invalid BNB Chain RPC URL: {0}")]
    InvalidRpcUrl(#[from] url::ParseError),
    #[error("Venus dependencies are unavailable")]
    Unavailable,
    #[error("BNB Chain RPC reported unexpected chain ID {chain_id}")]
    UnexpectedChain { chain_id: u64 },
}
