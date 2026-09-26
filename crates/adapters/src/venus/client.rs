use std::{sync::Arc, time::Duration};

use alloy_primitives::Address;
use reqwest_middleware::ClientBuilder;
use venus_api::apis::configuration::Configuration;

use crate::binance_web3::BinanceWeb3Client;

pub struct VenusBorrowingReader {
    pub(super) api: Configuration,
    pub(super) comptroller: Address,
    pub(super) binance: Arc<BinanceWeb3Client>,
}

impl VenusBorrowingReader {
    /// Creates the combined Venus market and account reader.
    ///
    /// # Errors
    ///
    /// Returns an error when the Venus HTTP client cannot be built.
    pub fn new(
        api_base_url: &str,
        comptroller: Address,
        binance: Arc<BinanceWeb3Client>,
        timeout: Duration,
    ) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        let api = Configuration {
            base_path: api_base_url.trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client: ClientBuilder::new(client).build(),
            ..Configuration::default()
        };

        Ok(Self {
            api,
            comptroller,
            binance,
        })
    }
}
