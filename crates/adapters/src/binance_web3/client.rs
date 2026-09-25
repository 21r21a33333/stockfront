use std::time::Duration;

use binance_web3_rwa::apis::configuration::Configuration;
use reqwest_middleware::ClientBuilder;

use super::auth::{BinanceConfigurationError, BinanceWeb3Authentication};

pub struct BinanceWeb3Client {
    pub(super) configuration: Configuration,
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
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        let client = ClientBuilder::new(client).with(authentication).build();
        let configuration = Configuration {
            base_path: base_url.trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client,
            ..Configuration::default()
        };
        Ok(Self { configuration })
    }
}

pub(super) fn is_business_success(code: Option<i32>, success: Option<bool>) -> bool {
    code == Some(0) && success != Some(false)
}
