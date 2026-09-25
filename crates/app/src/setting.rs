use std::{env, fmt, net::SocketAddr};

use serde::{
    Deserialize,
    de::{self, IntoDeserializer, Visitor, value::MapDeserializer},
};
use thiserror::Error;

const BINANCE_WEB3_API_KEY: &str = "#ENV:BINANCE_WEB3_API_KEY";
const BINANCE_WEB3_API_SECRET: &str = "#ENV:BINANCE_WEB3_API_SECRET";
const DEFAULT_BINANCE_WEB3_BASE_URL: &str = "https://web3.binance.com/build";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";

#[derive(Deserialize)]
pub(super) struct Settings {
    #[serde(deserialize_with = "deserialize_env_field")]
    pub(super) binance_web3_api_key: String,
    #[serde(deserialize_with = "deserialize_env_field")]
    pub(super) binance_web3_api_secret: String,
    pub(super) binance_web3_base_url: String,
    pub(super) bind_address: SocketAddr,
}

impl Settings {
    pub(super) fn from_env() -> Result<Self, SettingsError> {
        let binance_web3_base_url = env::var("BINANCE_WEB3_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BINANCE_WEB3_BASE_URL.to_owned());
        let bind_address =
            env::var("STOCKFRONT_BIND_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_owned());
        let fields = [
            ("binance_web3_api_key", BINANCE_WEB3_API_KEY),
            ("binance_web3_api_secret", BINANCE_WEB3_API_SECRET),
            ("binance_web3_base_url", binance_web3_base_url.as_str()),
            ("bind_address", bind_address.as_str()),
        ];
        let values = fields
            .into_iter()
            .map(|(key, value)| (key.into_deserializer(), value.into_deserializer()));

        Self::deserialize(MapDeserializer::<_, de::value::Error>::new(values))
            .map_err(|error| SettingsError::Deserialize(error.to_string()))
    }
}

#[derive(Debug, Error)]
pub(super) enum SettingsError {
    #[error("failed to load application settings: {0}")]
    Deserialize(String),
}

/// Resolves strings beginning with `#ENV:` from the process environment.
fn deserialize_env_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct EnvironmentVisitor;

    impl Visitor<'_> for EnvironmentVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string or an environment reference beginning with #ENV:")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value.strip_prefix("#ENV:") {
                Some(environment_name) => env::var(environment_name).map_err(|error| {
                    de::Error::custom(format_args!(
                        "environment variable {environment_name} is unavailable: {error}"
                    ))
                }),
                None => Ok(value.to_owned()),
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_str(EnvironmentVisitor)
}
