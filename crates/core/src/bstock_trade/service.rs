use std::sync::Arc;

use alloy_primitives::Address;
use thiserror::Error;

use super::{
    model::{
        ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitTradeRequest,
        SubmittedTrade, TradeQuote, TradeRequest, TradeStatus,
    },
    policy::TradePolicy,
    ports::{BstockTradeGateway, TradeGatewayError},
};

#[derive(Debug, Error)]
pub enum BstockTradeError {
    #[error(transparent)]
    Gateway(#[from] TradeGatewayError),
    #[error("the settlement token is not supported")]
    UnsupportedSettlementToken,
    #[error("no RFQ quote satisfies the configured price-impact limit")]
    NoSupportedQuote,
}

pub struct BstockTradeService {
    gateway: Arc<dyn BstockTradeGateway>,
    policy: TradePolicy,
}

impl BstockTradeService {
    #[must_use]
    pub fn new(gateway: Arc<dyn BstockTradeGateway>, policy: TradePolicy) -> Self {
        Self { gateway, policy }
    }

    /// Returns Binance RFQ quotes that satisfy Stockfront's price-impact policy.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when the request violates local policy or Binance is
    /// unavailable.
    pub async fn quotes(
        &self,
        request: &TradeRequest,
    ) -> Result<Vec<TradeQuote>, BstockTradeError> {
        self.validate_settlement_token(request.settlement_token)?;
        let mut quotes = self.gateway.quotes(request).await?;
        quotes.retain(|quote| self.policy.accepts_price_impact(quote.price_impact_percent));
        if quotes.is_empty() {
            return Err(BstockTradeError::NoSupportedQuote);
        }
        quotes.sort_by_key(|quote| std::cmp::Reverse(quote.raw_bstock_amount));
        Ok(quotes)
    }

    /// Returns Binance's exact approval transaction for the requested RFQ venue.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when the request violates local policy or Binance is
    /// unavailable.
    pub async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, BstockTradeError> {
        self.validate_settlement_token(request.settlement_token)?;
        self.gateway
            .prepare_approval(request)
            .await
            .map_err(Into::into)
    }

    /// Returns Binance's EIP-712 payload for the selected quote.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when the request violates local policy or Binance rejects the
    /// quote.
    pub async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
    ) -> Result<PreparedTrade, BstockTradeError> {
        self.validate_settlement_token(request.trade.settlement_token)?;
        self.gateway
            .prepare_trade(request, self.policy.slippage_percent)
            .await
            .map_err(Into::into)
    }

    /// Submits a wallet-signed RFQ order to Binance.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when Binance rejects the order.
    pub async fn submit_trade(
        &self,
        request: &SubmitTradeRequest,
    ) -> Result<SubmittedTrade, BstockTradeError> {
        self.gateway.submit_trade(request).await.map_err(Into::into)
    }

    /// Returns Binance's current RFQ order state.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when Binance is unavailable.
    pub async fn trade_status(&self, order_id: &str) -> Result<TradeStatus, BstockTradeError> {
        self.gateway
            .trade_status(order_id)
            .await
            .map_err(Into::into)
    }

    fn validate_settlement_token(&self, token: Address) -> Result<(), BstockTradeError> {
        if !self.policy.settlement_tokens.contains(&token) {
            return Err(BstockTradeError::UnsupportedSettlementToken);
        }
        Ok(())
    }
}
