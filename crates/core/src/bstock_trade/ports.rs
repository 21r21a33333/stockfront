use alloy_primitives::B256;
use async_trait::async_trait;
use rust_decimal::Decimal;
use thiserror::Error;

use super::model::{
    ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitOrderRequest,
    SubmittedOrder, TradeOrderStatus, TradeQuote, TradeQuoteRequest, TradeTransactionStatus,
};

#[derive(Debug, Error)]
pub enum TradeGatewayError {
    #[error("the trading provider is temporarily unavailable")]
    Unavailable,
    #[error("the trading provider rejected the request")]
    Rejected,
    #[error("the trading provider returned invalid data")]
    InvalidData,
    #[error("the trade quote expired")]
    QuoteExpired,
    #[error("the trade no longer matches its quote")]
    QuoteMismatch,
    #[error("the trade order was not found")]
    OrderNotFound,
    #[error("the trade transaction was not found")]
    TransactionNotFound,
}

#[async_trait]
pub trait BstockTradeGateway: Send + Sync {
    async fn quotes(
        &self,
        request: &TradeQuoteRequest,
    ) -> Result<Vec<TradeQuote>, TradeGatewayError>;

    async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeGatewayError>;

    async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
        slippage_percent: Decimal,
    ) -> Result<PreparedTrade, TradeGatewayError>;

    async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, TradeGatewayError>;

    async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, TradeGatewayError>;

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<TradeTransactionStatus, TradeGatewayError>;
}
