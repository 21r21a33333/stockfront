use std::borrow::Cow;

use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use stockfront_core::bstock_market::{BstockMarketError, CatalogError, QueryError};
use stockfront_core::bstock_trade::{BstockTradeError, TradeGatewayError};
use stockfront_core::wallet_portfolio::{
    BalanceReaderError, WalletAddressError, WalletPortfolioError,
};

use super::primitives::Response;

#[derive(Debug)]
pub(super) struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: Cow<'static, str>,
}

impl ApiError {
    pub(super) const INVALID_CONTRACT_ADDRESS: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "invalid_address",
        "contract must be an EVM address",
    );
    pub(super) const INVALID_WALLET_ADDRESS: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "invalid_wallet_address",
        "wallet must be a non-zero EVM address",
    );
    pub(super) const INVALID_TRANSACTION_HASH: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "invalid_transaction_hash",
        "transaction hash must be a 32-byte hex value",
    );
    pub(super) const BSTOCK_NOT_FOUND: Self = Self::fixed(
        StatusCode::NOT_FOUND,
        "bstock_not_found",
        "bStock was not found",
    );
    const UNSUPPORTED_CHAIN: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "unsupported_chain",
        "bStocks are only supported on BNB Chain (56)",
    );
    const MARKET_DATA_UNAVAILABLE: Self = Self::fixed(
        StatusCode::BAD_GATEWAY,
        "market_data_unavailable",
        "bStock market data is temporarily unavailable",
    );
    const INVALID_MARKET_DATA: Self = Self::fixed(
        StatusCode::BAD_GATEWAY,
        "invalid_market_data",
        "the market data provider returned an invalid response",
    );
    const BALANCE_DATA_UNAVAILABLE: Self = Self::fixed(
        StatusCode::BAD_GATEWAY,
        "balance_data_unavailable",
        "wallet balance data is temporarily unavailable",
    );
    const INVALID_BALANCE_DATA: Self = Self::fixed(
        StatusCode::BAD_GATEWAY,
        "invalid_balance_data",
        "the wallet balance provider returned an invalid response",
    );
    const TRADE_UNAVAILABLE: Self = Self::fixed(
        StatusCode::SERVICE_UNAVAILABLE,
        "trade_provider_unavailable",
        "bStock trading is temporarily unavailable",
    );
    const TRADE_REJECTED: Self = Self::fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        "trade_rejected",
        "the trading provider rejected the request",
    );
    const INVALID_TRADE_AMOUNT: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "invalid_trade_amount",
        "trade amount must be greater than zero",
    );
    const INVALID_TRADE_DATA: Self = Self::fixed(
        StatusCode::BAD_GATEWAY,
        "invalid_provider_response",
        "the trade provider returned invalid data",
    );
    const QUOTE_EXPIRED: Self = Self::fixed(
        StatusCode::CONFLICT,
        "quote_expired",
        "the quote has expired",
    );
    const QUOTE_MISMATCH: Self = Self::fixed(
        StatusCode::CONFLICT,
        "quote_mismatch",
        "the prepared trade no longer matches the quote",
    );
    const NO_EXECUTABLE_QUOTE: Self = Self::fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        "no_executable_quote",
        "no quote satisfies the trading policy",
    );
    const UNSUPPORTED_TRADE_PAIR: Self = Self::fixed(
        StatusCode::BAD_REQUEST,
        "unsupported_trade_pair",
        "the trade pair is not supported",
    );
    const TRADE_ORDER_NOT_FOUND: Self = Self::fixed(
        StatusCode::NOT_FOUND,
        "trade_order_not_found",
        "the trade order was not found",
    );
    const TRADE_TRANSACTION_NOT_FOUND: Self = Self::fixed(
        StatusCode::NOT_FOUND,
        "trade_transaction_not_found",
        "the trade transaction was not found",
    );
    const fn fixed(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message: Cow::Borrowed(message),
        }
    }

    fn dynamic(status: StatusCode, code: &'static str, message: String) -> Self {
        Self::new(status, code, Cow::Owned(message))
    }

    const fn new(status: StatusCode, code: &'static str, message: Cow<'static, str>) -> Self {
        Self {
            status,
            code,
            message,
        }
    }
}

impl From<QueryError> for ApiError {
    fn from(error: QueryError) -> Self {
        Self::dynamic(StatusCode::BAD_REQUEST, "invalid_query", error.to_string())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(error: JsonRejection) -> Self {
        Self::dynamic(
            StatusCode::BAD_REQUEST,
            "invalid_trade_request",
            error.body_text(),
        )
    }
}

impl From<BstockMarketError> for ApiError {
    fn from(error: BstockMarketError) -> Self {
        match error {
            BstockMarketError::UnsupportedChain => Self::UNSUPPORTED_CHAIN,
            BstockMarketError::Catalog(CatalogError::Unavailable) => Self::MARKET_DATA_UNAVAILABLE,
            BstockMarketError::Catalog(CatalogError::InvalidData { .. }) => {
                Self::INVALID_MARKET_DATA
            }
        }
    }
}

impl From<WalletAddressError> for ApiError {
    fn from(_error: WalletAddressError) -> Self {
        Self::INVALID_WALLET_ADDRESS
    }
}

impl From<WalletPortfolioError> for ApiError {
    fn from(error: WalletPortfolioError) -> Self {
        match error {
            WalletPortfolioError::Catalog(CatalogError::Unavailable) => {
                Self::MARKET_DATA_UNAVAILABLE
            }
            WalletPortfolioError::BalanceReader(BalanceReaderError::Unavailable) => {
                Self::BALANCE_DATA_UNAVAILABLE
            }
            WalletPortfolioError::Catalog(CatalogError::InvalidData { .. }) => {
                Self::INVALID_MARKET_DATA
            }
            WalletPortfolioError::BalanceReader(BalanceReaderError::InvalidData)
            | WalletPortfolioError::InvalidSnapshot => Self::INVALID_BALANCE_DATA,
        }
    }
}

impl From<BstockTradeError> for ApiError {
    fn from(error: BstockTradeError) -> Self {
        match error {
            BstockTradeError::Gateway(TradeGatewayError::Unavailable) => Self::TRADE_UNAVAILABLE,
            BstockTradeError::Gateway(TradeGatewayError::Rejected) => Self::TRADE_REJECTED,
            BstockTradeError::Gateway(TradeGatewayError::InvalidData) => Self::INVALID_TRADE_DATA,
            BstockTradeError::Gateway(TradeGatewayError::QuoteExpired) => Self::QUOTE_EXPIRED,
            BstockTradeError::Gateway(TradeGatewayError::QuoteMismatch) => Self::QUOTE_MISMATCH,
            BstockTradeError::NoSupportedQuote => Self::NO_EXECUTABLE_QUOTE,
            BstockTradeError::UnsupportedTradePair => Self::UNSUPPORTED_TRADE_PAIR,
            BstockTradeError::InvalidAmount => Self::INVALID_TRADE_AMOUNT,
            BstockTradeError::Catalog(CatalogError::Unavailable) => Self::MARKET_DATA_UNAVAILABLE,
            BstockTradeError::Catalog(CatalogError::InvalidData { .. }) => {
                Self::INVALID_MARKET_DATA
            }
            BstockTradeError::Gateway(TradeGatewayError::OrderNotFound) => {
                Self::TRADE_ORDER_NOT_FOUND
            }
            BstockTradeError::Gateway(TradeGatewayError::TransactionNotFound) => {
                Self::TRADE_TRANSACTION_NOT_FOUND
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        Response::error(self.code, self.message.into_owned(), self.status).into_response()
    }
}
