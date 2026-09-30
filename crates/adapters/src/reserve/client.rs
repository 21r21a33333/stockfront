use std::time::Duration;

use reqwest::Client;
use stockfront_core::baskets::BasketProviderError;
use thiserror::Error;
use url::Url;

use super::model::ReserveDtf;

pub struct ReserveClient {
    client: Client,
    discovery_url: Url,
}

impl ReserveClient {
    pub fn new(base_url: &str, timeout: Duration) -> Result<Self, ReserveConfigurationError> {
        let mut base_url = Url::parse(base_url)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let discovery_url = base_url.join("discover/dtfs")?;
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("stockfront/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| ReserveConfigurationError::Unavailable)?;
        Ok(Self {
            client,
            discovery_url,
        })
    }

    pub(super) async fn discover(&self) -> Result<Vec<ReserveDtf>, BasketProviderError> {
        let response = self
            .client
            .get(self.discovery_url.clone())
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
}

#[derive(Debug, Error)]
pub enum ReserveConfigurationError {
    #[error("invalid Reserve API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the Reserve API client is unavailable")]
    Unavailable,
}
