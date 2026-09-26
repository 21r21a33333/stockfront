use std::str::FromStr;

use alloy_primitives::U256;
use stockfront_core::{
    bstock_borrowing::BorrowingProviderError, bstock_trade::TradeGatewayError,
    wallet_portfolio::BalanceReaderError,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct InvalidProviderData;

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
pub(super) enum BinanceRequestError {
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
