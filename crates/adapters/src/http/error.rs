use axum::{
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use stockfront_core::bstock_market::{BstockMarketError, CatalogError, QueryError};

use super::primitives::Response;

#[derive(Debug)]
pub(super) enum ApiError {
    InvalidQuery(QueryError),
    InvalidAddress,
    UnsupportedChain,
    ProviderUnavailable,
    ProviderData,
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
            BstockMarketError::Catalog(CatalogError::Unavailable) => Self::ProviderUnavailable,
            BstockMarketError::Catalog(CatalogError::InvalidData { .. }) => Self::ProviderData,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        let (status, code, message) = match self {
            Self::InvalidQuery(error) => {
                (StatusCode::BAD_REQUEST, "invalid_query", error.to_string())
            }
            Self::InvalidAddress => (
                StatusCode::BAD_REQUEST,
                "invalid_address",
                "contract must be an EVM address".to_owned(),
            ),
            Self::UnsupportedChain => (
                StatusCode::BAD_REQUEST,
                "unsupported_chain",
                "bStocks are only supported on BNB Chain (56)".to_owned(),
            ),
            Self::ProviderUnavailable => (
                StatusCode::BAD_GATEWAY,
                "market_data_unavailable",
                "bStock market data is temporarily unavailable".to_owned(),
            ),
            Self::ProviderData => (
                StatusCode::BAD_GATEWAY,
                "invalid_market_data",
                "the market data provider returned an invalid response".to_owned(),
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
