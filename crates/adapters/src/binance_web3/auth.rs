use std::sync::Arc;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use http::Extensions;
use reqwest::{Request, Response, header::HeaderValue};
use reqwest_middleware::{Middleware, Next};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroizing;

const RECEIVE_WINDOW_MS: &str = "15000";

#[derive(Clone)]
pub(super) struct BinanceWeb3Authentication {
    api_key: HeaderValue,
    api_secret: Arc<Zeroizing<Vec<u8>>>,
}

impl BinanceWeb3Authentication {
    pub(super) fn new(api_key: &str, api_secret: &str) -> Result<Self, BinanceConfigurationError> {
        if api_key.is_empty() {
            return Err(BinanceConfigurationError::EmptyApiKey);
        }
        if api_secret.is_empty() {
            return Err(BinanceConfigurationError::EmptyApiSecret);
        }
        let api_key =
            HeaderValue::from_str(api_key).map_err(|_| BinanceConfigurationError::InvalidApiKey)?;
        Ok(Self {
            api_key,
            api_secret: Arc::new(Zeroizing::new(api_secret.as_bytes().to_vec())),
        })
    }

    fn authenticate(
        &self,
        request: &mut Request,
        timestamp: &str,
        nonce: &str,
    ) -> Result<(), AuthenticationError> {
        if request.method() != reqwest::Method::GET {
            return Err(AuthenticationError::UnsupportedMethod);
        }

        let mut path_and_query = request.url().path().to_owned();
        if let Some(query) = request.url().query() {
            path_and_query.push('?');
            path_and_query.push_str(query);
        }
        let prehash = format!("{timestamp}{}{path_and_query}", request.method().as_str());
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_slice())
            .map_err(|_| AuthenticationError::InvalidSecret)?;
        mac.update(prehash.as_bytes());
        let signature = BASE64_STANDARD.encode(mac.finalize().into_bytes());

        let headers = request.headers_mut();
        headers.insert("x-oc-apikey", self.api_key.clone());
        headers.insert("x-oc-timestamp", header_value(timestamp)?);
        headers.insert("x-oc-sign", header_value(&signature)?);
        headers.insert("x-oc-recv-window", header_value(RECEIVE_WINDOW_MS)?);
        headers.insert("x-oc-nonce", header_value(nonce)?);
        Ok(())
    }
}

#[async_trait]
impl Middleware for BinanceWeb3Authentication {
    async fn handle(
        &self,
        mut request: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let nonce = Uuid::new_v4().to_string();
        self.authenticate(&mut request, &timestamp, &nonce)
            .map_err(reqwest_middleware::Error::middleware)?;
        next.run(request, extensions).await
    }
}

#[derive(Debug, Error)]
pub enum BinanceConfigurationError {
    #[error("Binance Web3 API key must not be empty")]
    EmptyApiKey,
    #[error("Binance Web3 API key is not a valid HTTP header value")]
    InvalidApiKey,
    #[error("Binance Web3 API secret must not be empty")]
    EmptyApiSecret,
    #[error("failed to configure the Binance Web3 HTTP client")]
    HttpClient(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
enum AuthenticationError {
    #[error("the bStock market signer only supports GET requests")]
    UnsupportedMethod,
    #[error("the Binance Web3 API secret could not initialize HMAC-SHA256")]
    InvalidSecret,
    #[error("a Binance Web3 authentication header was invalid")]
    InvalidHeader,
}

fn header_value(value: &str) -> Result<HeaderValue, AuthenticationError> {
    HeaderValue::from_str(value).map_err(|_| AuthenticationError::InvalidHeader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_matches_fixed_binance_web3_hmac_vector() {
        let signer = BinanceWeb3Authentication::new("test-key", "test-secret")
            .unwrap_or_else(|error| panic!("signer setup failed: {error}"));
        let mut request = reqwest::Client::new()
            .get("https://web3.binance.com/build/api/v1/dex/market/rwa/tokens?binanceChainId=56&platformId=bstock")
            .build()
            .unwrap_or_else(|error| panic!("request setup failed: {error}"));

        signer
            .authenticate(&mut request, "2026-06-03T10:20:30.123Z", "fixed-nonce")
            .unwrap_or_else(|error| panic!("request signing failed: {error}"));

        assert_eq!(
            request.headers().get("x-oc-sign"),
            Some(&HeaderValue::from_static(
                "eCAOUppgxnP2Jr9G30oJyEzXtuXULR4N/jwOy/gLwAQ="
            ))
        );
        assert_eq!(
            request.headers().get("x-oc-timestamp"),
            Some(&HeaderValue::from_static("2026-06-03T10:20:30.123Z"))
        );
        assert_eq!(
            request.headers().get("x-oc-nonce"),
            Some(&HeaderValue::from_static("fixed-nonce"))
        );
    }
}
