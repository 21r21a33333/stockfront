use axum::{
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use stockfront_core::bstock_market::{BstockMarketError, CatalogError, QueryError};
use stockfront_core::wallet_portfolio::{
    BalanceReaderError, WalletAddressError, WalletPortfolioError,
};

use super::primitives::Response;

#[derive(Debug)]
pub(super) enum ApiError {
    InvalidQuery(QueryError),
    InvalidContractAddress,
    InvalidWalletAddress,
    UnsupportedChain,
    MarketDataUnavailable,
    InvalidMarketData,
    BalanceDataUnavailable,
    InvalidBalanceData,
    NotFound,
}

impl From<QueryError> for ApiError {
    fn from(value: QueryError) -> Self {
        Self::InvalidQuery(value)
    }
}

impl From<BstockMarketError> for ApiError {
    fn from(value: BstockMarketError) -> Self {
        match value {
            BstockMarketError::UnsupportedChain => Self::UnsupportedChain,
            BstockMarketError::Catalog(CatalogError::Unavailable) => Self::MarketDataUnavailable,
            BstockMarketError::Catalog(CatalogError::InvalidData { .. }) => Self::InvalidMarketData,
        }
    }
}

impl From<WalletAddressError> for ApiError {
    fn from(_value: WalletAddressError) -> Self {
        Self::InvalidWalletAddress
    }
}

impl From<WalletPortfolioError> for ApiError {
    fn from(value: WalletPortfolioError) -> Self {
        match value {
            WalletPortfolioError::Catalog(CatalogError::Unavailable) => Self::MarketDataUnavailable,
            WalletPortfolioError::BalanceReader(BalanceReaderError::Unavailable) => {
                Self::BalanceDataUnavailable
            }
            WalletPortfolioError::Catalog(CatalogError::InvalidData { .. }) => {
                Self::InvalidMarketData
            }
            WalletPortfolioError::BalanceReader(BalanceReaderError::InvalidData)
            | WalletPortfolioError::InvalidSnapshot => Self::InvalidBalanceData,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        let (status, code, message) = match self {
            Self::InvalidQuery(error) => {
                (StatusCode::BAD_REQUEST, "invalid_query", error.to_string())
            }
            Self::InvalidContractAddress => (
                StatusCode::BAD_REQUEST,
                "invalid_address",
                "contract must be an EVM address".to_owned(),
            ),
            Self::InvalidWalletAddress => (
                StatusCode::BAD_REQUEST,
                "invalid_wallet_address",
                "wallet must be a non-zero EVM address".to_owned(),
            ),
            Self::UnsupportedChain => (
                StatusCode::BAD_REQUEST,
                "unsupported_chain",
                "bStocks are only supported on BNB Chain (56)".to_owned(),
            ),
            Self::MarketDataUnavailable => (
                StatusCode::BAD_GATEWAY,
                "market_data_unavailable",
                "bStock market data is temporarily unavailable".to_owned(),
            ),
            Self::InvalidMarketData => (
                StatusCode::BAD_GATEWAY,
                "invalid_market_data",
                "the market data provider returned an invalid response".to_owned(),
            ),
            Self::BalanceDataUnavailable => (
                StatusCode::BAD_GATEWAY,
                "balance_data_unavailable",
                "wallet balance data is temporarily unavailable".to_owned(),
            ),
            Self::InvalidBalanceData => (
                StatusCode::BAD_GATEWAY,
                "invalid_balance_data",
                "the wallet balance provider returned an invalid response".to_owned(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "bstock_not_found",
                "bStock was not found".to_owned(),
            ),
        };
        Response::error(code, message, status).into_response()
    }
}
