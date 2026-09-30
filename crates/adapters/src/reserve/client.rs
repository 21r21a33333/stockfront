use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use stockfront_core::baskets::BasketProviderError;
use thiserror::Error;
use url::Url;

use super::model::ReserveDtf;

pub struct ReserveClient {
    pub(super) client: Client,
    pub(super) base_url: Url,
}

impl ReserveClient {
    pub fn new(base_url: &str, timeout: Duration) -> Result<Self, ReserveConfigurationError> {
        let mut base_url = Url::parse(base_url)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("stockfront/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| ReserveConfigurationError::Unavailable)?;
        Ok(Self { client, base_url })
    }

    pub(super) async fn discover(&self) -> Result<Vec<ReserveDtf>, BasketProviderError> {
        let response = self
            .get("discover/dtfs")
            .map_err(|_| BasketProviderError::InvalidData)?
            .query(&[
                ("chainId", "56"),
                ("brand", "true"),
                ("performance", "false"),
                ("limit", "100"),
            ])
            .send()
            .await
            .map_err(|_| BasketProviderError::Unavailable)?
            .error_for_status()
            .map_err(|_| BasketProviderError::Unavailable)?;
        response
            .json()
            .await
            .map_err(|_| BasketProviderError::InvalidData)
    }

    pub(super) fn get(&self, path: &str) -> Result<RequestBuilder, url::ParseError> {
        self.base_url.join(path).map(|url| self.client.get(url))
    }

    pub(super) fn post(&self, path: &str) -> Result<RequestBuilder, url::ParseError> {
        self.base_url.join(path).map(|url| self.client.post(url))
    }
}

#[derive(Debug, Error)]
pub enum ReserveConfigurationError {
    #[error("invalid Reserve API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the Reserve API client is unavailable")]
    Unavailable,
    #[error("Reserve 1inch Fusion trade slippage must be between 1 and 10000 basis points")]
    InvalidSlippage,
}
