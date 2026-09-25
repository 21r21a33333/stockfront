use std::{env, fmt, net::SocketAddr};

use serde::{
    Deserialize,
    de::{self, IntoDeserializer, Visitor, value::MapDeserializer},
};
use stockfront_core::bstock_market::Address;
use thiserror::Error;

const BINANCE_WEB3_API_KEY: &str = "#ENV:BINANCE_WEB3_API_KEY";
const BINANCE_WEB3_API_SECRET: &str = "#ENV:BINANCE_WEB3_API_SECRET";
const DEFAULT_BINANCE_WEB3_BASE_URL: &str = "https://web3.binance.com/build";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";
const DEFAULT_TRADE_SETTLEMENT_TOKENS: &str =
    "0x55d398326f99059ff775485246999027b3197955,0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d";
const DEFAULT_MAX_TRADE_PRICE_IMPACT_BPS: &str = "100";
const DEFAULT_TRADE_SLIPPAGE_BPS: &str = "50";

#[derive(Deserialize)]
pub(super) struct Settings {
    #[serde(deserialize_with = "deserialize_env_field")]
    pub(super) binance_web3_api_key: String,
    #[serde(deserialize_with = "deserialize_env_field")]
    pub(super) binance_web3_api_secret: String,
    pub(super) binance_web3_base_url: String,
    pub(super) bind_address: SocketAddr,
    #[serde(deserialize_with = "deserialize_addresses")]
    pub(super) trade_settlement_tokens: Vec<Address>,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub(super) max_trade_price_impact_bps: u32,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub(super) trade_slippage_bps: u32,
}

impl Settings {
    pub(super) fn from_env() -> Result<Self, SettingsError> {
        let binance_web3_base_url = env::var("BINANCE_WEB3_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BINANCE_WEB3_BASE_URL.to_owned());
        let bind_address =
            env::var("STOCKFRONT_BIND_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_owned());
        let trade_settlement_tokens = env::var("STOCKFRONT_TRADE_SETTLEMENT_TOKENS")
            .unwrap_or_else(|_| DEFAULT_TRADE_SETTLEMENT_TOKENS.to_owned());
        let max_trade_price_impact_bps = env::var("STOCKFRONT_MAX_TRADE_PRICE_IMPACT_BPS")
            .unwrap_or_else(|_| DEFAULT_MAX_TRADE_PRICE_IMPACT_BPS.to_owned());
        let trade_slippage_bps = env::var("STOCKFRONT_TRADE_SLIPPAGE_BPS")
            .unwrap_or_else(|_| DEFAULT_TRADE_SLIPPAGE_BPS.to_owned());
        let fields = [
            ("binance_web3_api_key", BINANCE_WEB3_API_KEY),
            ("binance_web3_api_secret", BINANCE_WEB3_API_SECRET),
            ("binance_web3_base_url", binance_web3_base_url.as_str()),
            ("bind_address", bind_address.as_str()),
            ("trade_settlement_tokens", trade_settlement_tokens.as_str()),
            (
                "max_trade_price_impact_bps",
                max_trade_price_impact_bps.as_str(),
            ),
            ("trade_slippage_bps", trade_slippage_bps.as_str()),
        ];
        let values = fields
            .into_iter()
            .map(|(key, value)| (key.into_deserializer(), value.into_deserializer()));

        Self::deserialize(MapDeserializer::<_, de::value::Error>::new(values))
            .map_err(|error| SettingsError::Deserialize(error.to_string()))
    }
}

fn deserialize_addresses<'de, D>(deserializer: D) -> Result<Vec<Address>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    value
        .split(',')
        .map(str::trim)
        .map(|address| {
            address
                .parse()
                .map_err(|_| de::Error::custom(format_args!("invalid EVM address: {address}")))
        })
        .collect()
}

fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: fmt::Display,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
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
