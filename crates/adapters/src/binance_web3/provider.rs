use std::{future::Future, str::FromStr, time::Duration};

use alloy_primitives::U256;
use stockfront_core::{
    bstock_borrowing::BorrowingProviderError, bstock_trade::TradeGatewayError,
    carry::EarnProviderError, liquidity::LiquidityProviderError,
    wallet_portfolio::BalanceReaderError,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct InvalidProviderData;

impl From<InvalidProviderData> for BorrowingProviderError {
    fn from(_error: InvalidProviderData) -> Self {
        Self::InvalidData
    }
}

impl From<InvalidProviderData> for TradeGatewayError {
    fn from(_error: InvalidProviderData) -> Self {
        Self::InvalidData
    }
}

impl From<InvalidProviderData> for BalanceReaderError {
    fn from(_error: InvalidProviderData) -> Self {
        Self::InvalidData
    }
}

impl From<InvalidProviderData> for EarnProviderError {
    fn from(_error: InvalidProviderData) -> Self {
        Self::InvalidData
    }
}

impl From<InvalidProviderData> for LiquidityProviderError {
    fn from(_error: InvalidProviderData) -> Self {
        Self::InvalidData
    }
}

pub(super) fn required<T>(value: Option<T>) -> Result<T, InvalidProviderData> {
    value.ok_or(InvalidProviderData)
}

pub(super) fn required_non_empty(value: Option<String>) -> Result<String, InvalidProviderData> {
    value
        .filter(|value| !value.is_empty())
        .ok_or(InvalidProviderData)
}

pub(super) fn parse<T>(value: &str) -> Result<T, InvalidProviderData>
where
    T: FromStr,
{
    value.parse().map_err(|_| InvalidProviderData)
}

pub(super) fn parse_optional<T>(value: Option<&str>) -> Result<Option<T>, InvalidProviderData>
where
    T: FromStr,
{
    value.map(parse).transpose()
}

pub(super) fn parse_u256(value: &str) -> Result<U256, InvalidProviderData> {
    if let Some(hex) = value.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|_| InvalidProviderData)
    } else {
        parse(value)
    }
}

pub(super) fn parse_optional_u256(
    value: Option<&str>,
) -> Result<Option<U256>, InvalidProviderData> {
    value.map(parse_u256).transpose()
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BinanceRequestError {
    Rejected,
    Unavailable,
}

impl From<BinanceRequestError> for BorrowingProviderError {
    fn from(error: BinanceRequestError) -> Self {
        match error {
            BinanceRequestError::Rejected => Self::Rejected,
            BinanceRequestError::Unavailable => Self::Unavailable,
        }
    }
}

impl From<BinanceRequestError> for TradeGatewayError {
    fn from(error: BinanceRequestError) -> Self {
        match error {
            BinanceRequestError::Rejected => Self::Rejected,
            BinanceRequestError::Unavailable => Self::Unavailable,
        }
    }
}

impl From<BinanceRequestError> for EarnProviderError {
    fn from(error: BinanceRequestError) -> Self {
        match error {
            BinanceRequestError::Rejected => Self::Rejected,
            BinanceRequestError::Unavailable => Self::Unavailable,
        }
    }
}

impl From<BinanceRequestError> for LiquidityProviderError {
    fn from(error: BinanceRequestError) -> Self {
        match error {
            BinanceRequestError::Rejected => Self::Rejected,
            BinanceRequestError::Unavailable => Self::Unavailable,
        }
    }
}

pub(super) fn business_succeeded(code: Option<i32>, success: Option<bool>) -> bool {
    code == Some(0) && success != Some(false)
}

pub(super) fn ensure_business_success(
    code: Option<i32>,
    success: Option<bool>,
) -> Result<(), BinanceRequestError> {
    if business_succeeded(code, success) {
        return Ok(());
    }

    match code {
        Some(40_000..=49_999) if !matches!(code, Some(40_101..=40_104 | 42_900)) => {
            Err(BinanceRequestError::Rejected)
        }
        _ => Err(BinanceRequestError::Unavailable),
    }
}

/// Retries a Binance read that failed for a reason other than an answer.
///
/// The `DeFi` endpoints fail intermittently — around three requests in ten while
/// testing earn — and a single failure otherwise becomes a 503 for the caller.
/// Only transport-level failures are retried; a rejection is the provider's
/// answer and repeating it would just be slower. The pause keeps a retry from
/// compounding a rate limit, which this mapping cannot distinguish.
pub(super) async fn retried<T, E, F, Fut>(mut call: F) -> Result<T, BinanceRequestError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, binance_web3_rwa::apis::Error<E>>>,
{
    const ATTEMPTS: u32 = 3;
    const BACKOFF: Duration = Duration::from_millis(200);
    for attempt in 1..ATTEMPTS {
        match call().await.map_err(map_request_error) {
            Err(BinanceRequestError::Unavailable) => {
                tokio::time::sleep(BACKOFF * attempt).await;
            }
            result => return result,
        }
    }
    call().await.map_err(map_request_error)
}

pub(super) fn map_request_error<T>(error: binance_web3_rwa::apis::Error<T>) -> BinanceRequestError {
    match error {
        binance_web3_rwa::apis::Error::ResponseError(response)
            if response.status.is_client_error()
                && !matches!(
                    response.status,
                    reqwest::StatusCode::UNAUTHORIZED
                        | reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::TOO_MANY_REQUESTS
                ) =>
        {
            BinanceRequestError::Rejected
        }
        _ => BinanceRequestError::Unavailable,
    }
}
