use std::{env, fmt, net::SocketAddr};

use serde::{
    Deserialize,
    de::{self, IntoDeserializer, Visitor, value::MapDeserializer},
};
use stockfront_core::bstock_market::Address;
use thiserror::Error;

const BINANCE_WEB3_API_KEY: &str = "#ENV:BINANCE_WEB3_API_KEY";
const BINANCE_WEB3_API_SECRET: &str = "#ENV:BINANCE_WEB3_API_SECRET";
const GLIDER_API_KEY: &str = "#ENV:GLIDER_API_KEY";
const GEMINI_API_KEY: &str = "#ENV:GEMINI_API_KEY";
const OPENROUTER_API_KEY: &str = "#ENV:OPENROUTER_API_KEY";
const DEFAULT_BINANCE_WEB3_BASE_URL: &str = "https://web3.binance.com/build";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";
const DEFAULT_TRADE_SETTLEMENT_TOKENS: &str =
    "0x55d398326f99059ff775485246999027b3197955,0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d";
const DEFAULT_MAX_TRADE_PRICE_IMPACT_BPS: &str = "100";
const DEFAULT_TRADE_SLIPPAGE_BPS: &str = "50";
const DEFAULT_VENUS_API_BASE_URL: &str = "https://api.venus.io";
const DEFAULT_BSC_RPC_URL: &str = "https://bsc-dataseed.bnbchain.org";
const DEFAULT_VENUS_COMPTROLLER: &str = "0xfD36E2c2a6789Db23113685031d7F16329158384";
const DEFAULT_BORROW_DEBT_TOKENS: &str = "0x55d398326f99059fF775485246999027B3197955";
const DEFAULT_CARRY_INVESTMENT_IDS: &str = "";
const DEFAULT_MIN_CARRY_SPREAD_BPS: &str = "100";
const DEFAULT_LIQUIDITY_INVESTMENT_IDS: &str = "";
const DEFAULT_LIQUIDITY_SLIPPAGE_BPS: &str = "100";
const DEFAULT_RESERVE_API_BASE_URL: &str = "https://api.reserve.org";
const DEFAULT_GLIDER_API_BASE_URL: &str = "https://api.glider.fi/v2";
const DEFAULT_GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_GEMINI_MODEL: &str = "gemini-flash-latest";
const DEFAULT_AI_PROVIDER: &str = "openrouter";
const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const DEFAULT_OPENROUTER_MODEL: &str = "google/gemini-2.5-flash";
const DEFAULT_OPENROUTER_REQUIRE_PARAMETERS: &str = "false";
const DEFAULT_AI_REQUEST_TIMEOUT_SECONDS: &str = "60";

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
    pub(super) venus_api_base_url: String,
    pub(super) bsc_rpc_url: String,
    #[serde(deserialize_with = "deserialize_address")]
    pub(super) venus_comptroller: Address,
    #[serde(deserialize_with = "deserialize_addresses")]
    pub(super) borrow_debt_tokens: Vec<Address>,
    #[serde(deserialize_with = "deserialize_strings")]
    pub(super) carry_investment_ids: Vec<String>,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub(super) min_carry_spread_bps: u32,
    #[serde(deserialize_with = "deserialize_strings")]
    pub(super) liquidity_investment_ids: Vec<String>,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub(super) liquidity_slippage_bps: u32,
    pub(super) reserve_api_base_url: String,
    #[serde(deserialize_with = "deserialize_env_field")]
    pub(super) glider_api_key: String,
    pub(super) glider_api_base_url: String,
    #[serde(flatten)]
    pub(super) ai: AiSettings,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum AiProvider {
    #[serde(rename = "openrouter")]
    OpenRouter,
    Gemini,
}

/// Both providers are configured so either can be selected without a restart
/// of the deployment's configuration, but only the chosen one needs a key.
#[derive(Deserialize)]
pub(super) struct AiSettings {
    #[serde(rename = "ai_provider")]
    pub(super) provider: AiProvider,
    #[serde(
        rename = "ai_request_timeout_seconds",
        deserialize_with = "deserialize_from_str"
    )]
    pub(super) request_timeout_seconds: u64,
    #[serde(
        rename = "gemini_api_key",
        deserialize_with = "deserialize_optional_env_field"
    )]
    pub(super) gemini_api_key: String,
    #[serde(rename = "gemini_api_base_url")]
    pub(super) gemini_api_base_url: String,
    #[serde(rename = "gemini_model")]
    pub(super) gemini_model: String,
    #[serde(
        rename = "openrouter_api_key",
        deserialize_with = "deserialize_optional_env_field"
    )]
    pub(super) openrouter_api_key: String,
    #[serde(rename = "openrouter_base_url")]
    pub(super) openrouter_base_url: String,
    #[serde(rename = "openrouter_model")]
    pub(super) openrouter_model: String,
    #[serde(
        rename = "openrouter_require_parameters",
        deserialize_with = "deserialize_from_str"
    )]
    pub(super) openrouter_require_parameters: bool,
}

impl Settings {
    pub(super) fn from_env() -> Result<Self, SettingsError> {
        // A local `.env` seeds the process environment; variables already
        // exported by the shell or the deployment win over the file.
        dotenvy::dotenv().ok();
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
        let venus_api_base_url = env::var("VENUS_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_VENUS_API_BASE_URL.to_owned());
        let bsc_rpc_url =
            env::var("BSC_RPC_URL").unwrap_or_else(|_| DEFAULT_BSC_RPC_URL.to_owned());
        let venus_comptroller = env::var("VENUS_COMPTROLLER_ADDRESS")
            .unwrap_or_else(|_| DEFAULT_VENUS_COMPTROLLER.to_owned());
        let borrow_debt_tokens = env::var("STOCKFRONT_BORROW_DEBT_TOKENS")
            .unwrap_or_else(|_| DEFAULT_BORROW_DEBT_TOKENS.to_owned());
        let carry_investment_ids = env::var("STOCKFRONT_CARRY_INVESTMENT_IDS")
            .unwrap_or_else(|_| DEFAULT_CARRY_INVESTMENT_IDS.to_owned());
        let min_carry_spread_bps = env::var("STOCKFRONT_MIN_CARRY_SPREAD_BPS")
            .unwrap_or_else(|_| DEFAULT_MIN_CARRY_SPREAD_BPS.to_owned());
        let liquidity_investment_ids = env::var("STOCKFRONT_LIQUIDITY_INVESTMENT_IDS")
            .unwrap_or_else(|_| DEFAULT_LIQUIDITY_INVESTMENT_IDS.to_owned());
        let liquidity_slippage_bps = env::var("STOCKFRONT_LIQUIDITY_SLIPPAGE_BPS")
            .unwrap_or_else(|_| DEFAULT_LIQUIDITY_SLIPPAGE_BPS.to_owned());
        let reserve_api_base_url = env::var("RESERVE_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_RESERVE_API_BASE_URL.to_owned());
        let glider_api_base_url = env::var("GLIDER_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_GLIDER_API_BASE_URL.to_owned());
        let gemini_api_base_url = env::var("GEMINI_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_GEMINI_API_BASE_URL.to_owned());
        let gemini_model =
            env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_GEMINI_MODEL.to_owned());
        let ai_provider =
            env::var("AI_PROVIDER").unwrap_or_else(|_| DEFAULT_AI_PROVIDER.to_owned());
        let ai_request_timeout_seconds = env::var("AI_REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| DEFAULT_AI_REQUEST_TIMEOUT_SECONDS.to_owned());
        let openrouter_base_url = env::var("OPENROUTER_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENROUTER_BASE_URL.to_owned());
        let openrouter_model =
            env::var("OPENROUTER_MODEL").unwrap_or_else(|_| DEFAULT_OPENROUTER_MODEL.to_owned());
        let openrouter_require_parameters = env::var("OPENROUTER_REQUIRE_PARAMETERS")
            .unwrap_or_else(|_| DEFAULT_OPENROUTER_REQUIRE_PARAMETERS.to_owned());
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
            ("venus_api_base_url", venus_api_base_url.as_str()),
            ("bsc_rpc_url", bsc_rpc_url.as_str()),
            ("venus_comptroller", venus_comptroller.as_str()),
            ("borrow_debt_tokens", borrow_debt_tokens.as_str()),
            ("carry_investment_ids", carry_investment_ids.as_str()),
            ("min_carry_spread_bps", min_carry_spread_bps.as_str()),
            (
                "liquidity_investment_ids",
                liquidity_investment_ids.as_str(),
            ),
            ("liquidity_slippage_bps", liquidity_slippage_bps.as_str()),
            ("reserve_api_base_url", reserve_api_base_url.as_str()),
            ("glider_api_key", GLIDER_API_KEY),
            ("glider_api_base_url", glider_api_base_url.as_str()),
            ("gemini_api_key", GEMINI_API_KEY),
            ("gemini_api_base_url", gemini_api_base_url.as_str()),
            ("gemini_model", gemini_model.as_str()),
            ("ai_provider", ai_provider.as_str()),
            (
                "ai_request_timeout_seconds",
                ai_request_timeout_seconds.as_str(),
            ),
            ("openrouter_api_key", OPENROUTER_API_KEY),
            ("openrouter_base_url", openrouter_base_url.as_str()),
            ("openrouter_model", openrouter_model.as_str()),
            (
                "openrouter_require_parameters",
                openrouter_require_parameters.as_str(),
            ),
        ];
        let values = fields
            .into_iter()
            .map(|(key, value)| (key.into_deserializer(), value.into_deserializer()));

        Self::deserialize(MapDeserializer::<_, de::value::Error>::new(values))
            .map_err(|error| SettingsError::Deserialize(error.to_string()))
    }
}

fn deserialize_strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)?
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect())
}

fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: de::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(|_| de::Error::custom("invalid EVM address"))
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
/// Like [`deserialize_env_field`] but yields an empty string when the variable
/// is unset, so an unused provider's key is not required to boot.
fn deserialize_optional_env_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct OptionalEnvironmentVisitor;

    impl Visitor<'_> for OptionalEnvironmentVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string or an environment reference beginning with #ENV:")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(match value.strip_prefix("#ENV:") {
                Some(environment_name) => env::var(environment_name).unwrap_or_default(),
                None => value.to_owned(),
            })
        }
    }

    deserializer.deserialize_str(OptionalEnvironmentVisitor)
}

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
