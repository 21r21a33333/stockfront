use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    Client, StatusCode,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use stockfront_core::strategy_composer::{
    ComposeContext, StrategyDraft, StrategyModel, StrategyModelError,
};
use thiserror::Error;
use url::Url;

use super::{request::ChatCompletionRequest, response::ChatCompletionResponse};

pub struct OpenAiCompatibleClient {
    client: Client,
    endpoint: Url,
    model: String,
    require_parameters: bool,
}

impl OpenAiCompatibleClient {
    /// Creates a client for one chat-completions model.
    ///
    /// `require_parameters` is an `OpenRouter` routing hint; leave it off for
    /// servers that reject unknown request fields.
    ///
    /// # Errors
    ///
    /// Returns [`OpenAiCompatibleConfigurationError`] for an invalid URL, model
    /// or key.
    pub fn new(
        base_url: &str,
        model: &str,
        api_key: &str,
        require_parameters: bool,
        timeout: Duration,
    ) -> Result<Self, OpenAiCompatibleConfigurationError> {
        let mut base_url = Url::parse(base_url)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let endpoint = base_url.join("chat/completions")?;
        if model.is_empty() {
            return Err(OpenAiCompatibleConfigurationError::InvalidModel);
        }

        let mut authorization = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .ok()
            .filter(|_| !api_key.is_empty())
            .ok_or(OpenAiCompatibleConfigurationError::InvalidApiKey)?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("stockfront/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()
            .map_err(|_| OpenAiCompatibleConfigurationError::Unavailable)?;
        Ok(Self {
            client,
            endpoint,
            model: model.to_owned(),
            require_parameters,
        })
    }
}

#[async_trait]
impl StrategyModel for OpenAiCompatibleClient {
    async fn draft(&self, context: &ComposeContext) -> Result<StrategyDraft, StrategyModelError> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .json(&ChatCompletionRequest::new(
                self.model.clone(),
                self.require_parameters,
                context,
            ))
            .send()
            .await
            .map_err(|_| StrategyModelError::Unavailable)?;
        match response.status() {
            status if status.is_success() => {}
            // Both exhausted quota and an empty balance are fixed by waiting or
            // topping up, so they share the rate-limited meaning.
            StatusCode::TOO_MANY_REQUESTS | StatusCode::PAYMENT_REQUIRED => {
                return Err(StrategyModelError::RateLimited);
            }
            _ => return Err(StrategyModelError::Unavailable),
        }
        response
            .json::<ChatCompletionResponse>()
            .await
            .map_err(|_| StrategyModelError::InvalidResponse)?
            .try_into()
    }
}

#[derive(Debug, Error)]
pub enum OpenAiCompatibleConfigurationError {
    #[error("invalid chat completions URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the model name is empty")]
    InvalidModel,
    #[error("the API key is empty or not a valid header value")]
    InvalidApiKey,
    #[error("the chat completions HTTP client could not be built")]
    Unavailable,
}
