use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use stockfront_core::strategy_composer::{
    ComposeContext, StrategyDraft, StrategyModel, StrategyModelError,
};
use thiserror::Error;
use url::Url;

use super::{request::GenerateContentRequest, response::GenerateContentResponse};

pub struct GeminiClient {
    client: Client,
    endpoint: Url,
}

impl GeminiClient {
    /// Creates a client for one Gemini model.
    ///
    /// # Errors
    ///
    /// Returns [`GeminiConfigurationError`] for an invalid URL, model or key.
    pub fn new(
        base_url: &str,
        model: &str,
        api_key: &str,
        timeout: Duration,
    ) -> Result<Self, GeminiConfigurationError> {
        let mut base_url = Url::parse(base_url)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        if model.is_empty() || model.contains(['/', ':', '?', '#']) {
            return Err(GeminiConfigurationError::InvalidModel);
        }
        let endpoint = base_url.join(&format!("models/{model}:generateContent"))?;

        let mut key = HeaderValue::from_str(api_key)
            .ok()
            .filter(|_| !api_key.is_empty())
            .ok_or(GeminiConfigurationError::InvalidApiKey)?;
        key.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("x-goog-api-key", key);
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("stockfront/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()
            .map_err(|_| GeminiConfigurationError::Unavailable)?;
        Ok(Self { client, endpoint })
    }
}

#[async_trait]
impl StrategyModel for GeminiClient {
    async fn draft(&self, context: &ComposeContext) -> Result<StrategyDraft, StrategyModelError> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .json(&GenerateContentRequest::from(context))
            .send()
            .await
            .map_err(|_| StrategyModelError::Unavailable)?;
        match response.status() {
            status if status.is_success() => {}
            StatusCode::TOO_MANY_REQUESTS => return Err(StrategyModelError::RateLimited),
            _ => return Err(StrategyModelError::Unavailable),
        }
        response
            .json::<GenerateContentResponse>()
            .await
            .map_err(|_| StrategyModelError::InvalidResponse)?
            .try_into()
    }
}

#[derive(Debug, Error)]
pub enum GeminiConfigurationError {
    #[error("invalid Gemini API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the Gemini model name is invalid")]
    InvalidModel,
    #[error("the Gemini API key is empty or not a valid header value")]
    InvalidApiKey,
    #[error("the Gemini HTTP client could not be built")]
    Unavailable,
}
