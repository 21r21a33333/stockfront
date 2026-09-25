use std::sync::Arc;

use thiserror::Error;

use super::{
    model::{
        ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitOrderRequest,
        SubmittedOrder, TradeOrderStatus, TradeQuote, TradeQuoteRequest,
    },
    policy::TradePolicy,
    ports::{BstockTradeGateway, TradeGatewayError},
};
use crate::bstock_market::{BNB_CHAIN_ID, BstockCatalog, BstockId, CatalogError};

#[derive(Debug, Error)]
pub enum BstockTradeError {
    #[error(transparent)]
    Gateway(#[from] TradeGatewayError),
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error("trade amount must be greater than zero")]
    InvalidAmount,
    #[error("the trade pair is not supported")]
    UnsupportedTradePair,
    #[error("no RFQ quote satisfies the configured price-impact limit")]
    NoSupportedQuote,
}

pub struct BstockTradeService {
    gateway: Arc<dyn BstockTradeGateway>,
    catalog: Arc<dyn BstockCatalog>,
    policy: TradePolicy,
}

impl BstockTradeService {
    #[must_use]
    pub fn new(
        gateway: Arc<dyn BstockTradeGateway>,
        catalog: Arc<dyn BstockCatalog>,
        policy: TradePolicy,
    ) -> Self {
        Self {
            gateway,
            catalog,
            policy,
        }
    }

    /// Returns Binance RFQ quotes that satisfy Stockfront's price-impact policy.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when the request violates local policy or Binance is
    /// unavailable.
    pub async fn quotes(
        &self,
        request: &TradeQuoteRequest,
    ) -> Result<Vec<TradeQuote>, BstockTradeError> {
        self.validate_trade(request).await?;
        let mut quotes = self.gateway.quotes(request).await?;
        quotes.retain(|quote| self.policy.accepts_price_impact(quote.price_impact_percent));
        if quotes.is_empty() {
            return Err(BstockTradeError::NoSupportedQuote);
        }
        quotes.sort_by_key(|quote| std::cmp::Reverse(quote.buy.amount));
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
        if request.amount.is_zero() {
            return Err(BstockTradeError::InvalidAmount);
        }
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
        self.validate_trade(&request.trade).await?;
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
    pub async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, BstockTradeError> {
        self.gateway.submit_order(request).await.map_err(Into::into)
    }

    /// Returns Binance's current RFQ order state.
    ///
    /// # Errors
    ///
    /// Returns [`BstockTradeError`] when Binance is unavailable.
    pub async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, BstockTradeError> {
        self.gateway
            .order_status(order_id)
            .await
            .map_err(Into::into)
    }

    async fn validate_trade(&self, request: &TradeQuoteRequest) -> Result<(), BstockTradeError> {
        if request.sell_amount.is_zero() {
            return Err(BstockTradeError::InvalidAmount);
        }
        let contract = self
            .policy
            .bstock_for_pair(request.sell_token, request.buy_token)
            .ok_or(BstockTradeError::UnsupportedTradePair)?;
        let bstock = BstockId {
            chain_id: BNB_CHAIN_ID,
            contract,
        };
        let supported = self
            .catalog
            .list_bstocks()
            .await?
            .iter()
            .any(|asset| asset.id == bstock);
        if supported {
            Ok(())
        } else {
            Err(BstockTradeError::UnsupportedTradePair)
        }
    }
}
