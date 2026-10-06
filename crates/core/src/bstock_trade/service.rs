use std::sync::Arc;

use alloy_primitives::{Address, B256};
use thiserror::Error;

use super::{
    model::{
        ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, SubmitOrderRequest,
        SubmittedOrder, TradeOrderStatus, TradeQuote, TradeQuoteRequest, TradeTransactionStatus,
    },
    policy::TradePolicy,
    ports::{TradeGateway, TradeGatewayError},
};
use crate::bstock_market::{BNB_CHAIN_ID, BstockCatalog, BstockId, CatalogError};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum TradeCatalogError {
    #[error("the trade asset catalog is temporarily unavailable")]
    Unavailable,
    #[error("the trade asset catalog returned invalid data")]
    InvalidData,
}

#[derive(Debug, Error)]
pub enum TradeError {
    #[error(transparent)]
    Gateway(#[from] TradeGatewayError),
    #[error(transparent)]
    Catalog(#[from] TradeCatalogError),
    #[error("trade amount must be greater than zero")]
    InvalidAmount,
    #[error("the trade pair is not supported")]
    UnsupportedTradePair,
    #[error("no quote satisfies the configured price-impact limit")]
    NoSupportedQuote,
}

pub struct BstockTradeService {
    gateway: Arc<dyn TradeGateway>,
    catalog: Arc<dyn BstockCatalog>,
    policy: TradePolicy,
}

impl BstockTradeService {
    #[must_use]
    pub fn new(
        gateway: Arc<dyn TradeGateway>,
        catalog: Arc<dyn BstockCatalog>,
        policy: TradePolicy,
    ) -> Self {
        Self {
            gateway,
            catalog,
            policy,
        }
    }

    /// Returns quotes that satisfy Stockfront's price-impact policy.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the request violates local policy or the provider is unavailable.
    pub async fn quotes(&self, request: &TradeQuoteRequest) -> Result<Vec<TradeQuote>, TradeError> {
        self.validate_trade(request).await?;
        select_quotes(self.gateway.quotes(request).await?, &self.policy)
    }

    /// Returns the provider's exact approval transaction for the requested venue.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the request violates local policy or the provider is unavailable.
    pub async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeError> {
        if request.amount.is_zero() {
            return Err(TradeError::InvalidAmount);
        }
        self.gateway
            .prepare_approval(request)
            .await
            .map_err(Into::into)
    }

    /// Returns a wallet transaction or EIP-712 payload for the selected quote.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the request violates local policy or the provider rejects the
    /// quote.
    pub async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
    ) -> Result<PreparedTrade, TradeError> {
        self.validate_trade(&request.trade).await?;
        self.gateway
            .prepare_trade(request, self.policy.slippage_percent)
            .await
            .map_err(Into::into)
    }

    /// Submits a wallet-signed RFQ order.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the provider rejects the order.
    pub async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, TradeError> {
        self.gateway.submit_order(request).await.map_err(Into::into)
    }

    /// Returns the provider's current RFQ order state.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the provider is unavailable.
    pub async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, TradeError> {
        self.gateway
            .order_status(order_id)
            .await
            .map_err(Into::into)
    }

    /// Returns the provider's current on-chain transaction state.
    ///
    /// # Errors
    ///
    /// Returns [`TradeError`] when the provider is unavailable or the transaction is unknown.
    pub async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<TradeTransactionStatus, TradeError> {
        self.gateway
            .transaction_status(transaction_hash)
            .await
            .map_err(Into::into)
    }

    #[must_use]
    pub fn supports_pair_shape(&self, sell_token: Address, buy_token: Address) -> bool {
        self.policy.asset_for_pair(sell_token, buy_token).is_some()
    }

    async fn validate_trade(&self, request: &TradeQuoteRequest) -> Result<(), TradeError> {
        if request.sell_amount.is_zero() {
            return Err(TradeError::InvalidAmount);
        }
        let contract = self
            .policy
            .asset_for_pair(request.sell_token, request.buy_token)
            .ok_or(TradeError::UnsupportedTradePair)?;
        let bstock = BstockId {
            chain_id: BNB_CHAIN_ID,
            contract,
        };
        let supported = self
            .catalog
            .list_bstocks()
            .await
            .map_err(map_bstock_catalog_error)?
            .iter()
            .any(|asset| asset.id == bstock);
        if supported {
            Ok(())
        } else {
            Err(TradeError::UnsupportedTradePair)
        }
    }
}

fn map_bstock_catalog_error(error: CatalogError) -> TradeCatalogError {
    match error {
        CatalogError::Unavailable => TradeCatalogError::Unavailable,
        CatalogError::InvalidData { .. } => TradeCatalogError::InvalidData,
    }
}

pub(crate) fn select_quotes(
    mut quotes: Vec<TradeQuote>,
    policy: &TradePolicy,
) -> Result<Vec<TradeQuote>, TradeError> {
    quotes.retain(|quote| {
        quote
            .price_impact_percent
            .is_none_or(|impact| policy.accepts_price_impact(impact))
    });
    if quotes.is_empty() {
        return Err(TradeError::NoSupportedQuote);
    }
    quotes.sort_by_key(|quote| std::cmp::Reverse(quote.buy.amount));
    Ok(quotes)
}
