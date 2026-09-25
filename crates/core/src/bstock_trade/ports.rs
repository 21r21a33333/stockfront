use async_trait::async_trait;
use rust_decimal::Decimal;
use thiserror::Error;

use super::model::{
    ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitTradeRequest,
    SubmittedTrade, TradeQuote, TradeRequest, TradeStatus,
};

#[derive(Debug, Error)]
pub enum TradeGatewayError {
    #[error("the trading provider is temporarily unavailable")]
    Unavailable,
    #[error("the trading provider returned invalid data")]
    InvalidData,
    #[error("the trade quote expired")]
    QuoteExpired,
    #[error("the trade no longer matches its quote")]
    QuoteMismatch,
    #[error("the trade order was not found")]
    OrderNotFound,
}

#[async_trait]
pub trait BstockTradeGateway: Send + Sync {
    async fn quotes(&self, request: &TradeRequest) -> Result<Vec<TradeQuote>, TradeGatewayError>;

    async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeGatewayError>;

    async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
        slippage_percent: Decimal,
    ) -> Result<PreparedTrade, TradeGatewayError>;

    async fn submit_trade(
        &self,
        request: &SubmitTradeRequest,
    ) -> Result<SubmittedTrade, TradeGatewayError>;

    async fn trade_status(&self, order_id: &str) -> Result<TradeStatus, TradeGatewayError>;
}
