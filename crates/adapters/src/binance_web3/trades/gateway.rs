use async_trait::async_trait;
use binance_web3_rwa::{
    apis::trading_api::{
        BuildSwapTransactionParams, GetAggregatedQuoteParams, GetErc20ApproveTransactionParams,
        GetRfqOrderStatusParams, SubmitRfqOrderParams, build_swap_transaction,
        get_aggregated_quote, get_erc20_approve_transaction, get_rfq_order_status,
        submit_rfq_order,
    },
    models,
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_market::BNB_CHAIN_ID,
    bstock_trade::{
        ApprovalRequest, BstockTradeGateway, PrepareTradeRequest, PreparedApproval, PreparedTrade,
        SubmitTradeRequest, SubmittedTrade, TradeGatewayError, TradeQuote, TradeRequest,
        TradeStatus,
    },
};

use super::{
    super::client::{BinanceWeb3Client, is_business_success},
    mapping::{
        ProviderApproval, ProviderPreparedTrade, ProviderQuote, ProviderSubmittedTrade,
        ProviderTradeStatus, provider_venue,
    },
};

#[async_trait]
impl BstockTradeGateway for BinanceWeb3Client {
    async fn quotes(&self, request: &TradeRequest) -> Result<Vec<TradeQuote>, TradeGatewayError> {
        let response = get_aggregated_quote(
            &self.configuration,
            GetAggregatedQuoteParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                amount: request.raw_settlement_amount.to_string(),
                from_token_address: request.settlement_token.to_string(),
                to_token_address: request.bstock_contract.to_string(),
                user_wallet_address: request.wallet.as_address().to_string(),
            },
        )
        .await
        .map_err(|_| TradeGatewayError::Unavailable)?;
        ensure_business_success(response.code, response.success)?;
        Ok(response
            .data
            .unwrap_or_default()
            .into_iter()
            .filter(|quote| quote.execution_mode.as_deref() == Some("RFQ"))
            .filter_map(|quote| TradeQuote::try_from(ProviderQuote(quote)).ok())
            .collect())
    }

    async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeGatewayError> {
        let response = get_erc20_approve_transaction(
            &self.configuration,
            GetErc20ApproveTransactionParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                token_contract_address: request.settlement_token.to_string(),
                approve_amount: request.raw_settlement_amount.to_string(),
                vendor: provider_venue(request.venue),
            },
        )
        .await
        .map_err(|_| TradeGatewayError::Unavailable)?;
        ensure_business_success(response.code, response.success)?;
        let approval = response
            .data
            .unwrap_or_default()
            .into_iter()
            .next()
            .ok_or(TradeGatewayError::InvalidData)?;
        PreparedApproval::try_from(ProviderApproval {
            token: request.settlement_token,
            value: approval,
        })
    }

    async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
        slippage_percent: Decimal,
    ) -> Result<PreparedTrade, TradeGatewayError> {
        let response = build_swap_transaction(
            &self.configuration,
            BuildSwapTransactionParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                amount: request.trade.raw_settlement_amount.to_string(),
                from_token_address: request.trade.settlement_token.to_string(),
                to_token_address: request.trade.bstock_contract.to_string(),
                user_wallet_address: request.trade.wallet.as_address().to_string(),
                quote_id: request.quote_id.clone(),
                slippage_percent: slippage_percent.to_string(),
            },
        )
        .await
        .map_err(|_| TradeGatewayError::Unavailable)?;
        ensure_quote_business_success(response.code, response.success)?;
        let prepared = response.data.ok_or(TradeGatewayError::InvalidData)?;
        Ok(ProviderPreparedTrade(*prepared.rfq).into())
    }

    async fn submit_trade(
        &self,
        request: &SubmitTradeRequest,
    ) -> Result<SubmittedTrade, TradeGatewayError> {
        let mut body = models::SubmitRfqOrderRequest::new(
            request.request_id,
            request.user_signature.clone(),
            provider_venue(request.venue),
            request.quote_id.clone(),
        );
        body.signing_scheme = Some(models::RfqSigningScheme::Eip712);
        let response = submit_rfq_order(
            &self.configuration,
            SubmitRfqOrderParams {
                submit_rfq_order_request: body,
            },
        )
        .await
        .map_err(|_| TradeGatewayError::Unavailable)?;
        ensure_quote_business_success(response.code, response.success)?;
        let submitted = response.data.ok_or(TradeGatewayError::InvalidData)?;
        Ok(ProviderSubmittedTrade(*submitted).into())
    }

    async fn trade_status(&self, order_id: &str) -> Result<TradeStatus, TradeGatewayError> {
        let response = get_rfq_order_status(
            &self.configuration,
            GetRfqOrderStatusParams {
                order_id: order_id.to_owned(),
            },
        )
        .await
        .map_err(|error| match error {
            binance_web3_rwa::apis::Error::ResponseError(response)
                if response.status == reqwest::StatusCode::NOT_FOUND =>
            {
                TradeGatewayError::OrderNotFound
            }
            _ => TradeGatewayError::Unavailable,
        })?;
        if response.code == Some(404) || response.code == Some(40_404) {
            return Err(TradeGatewayError::OrderNotFound);
        }
        ensure_business_success(response.code, response.success)?;
        let status = response.data.ok_or(TradeGatewayError::InvalidData)?;
        TradeStatus::try_from(ProviderTradeStatus(*status))
    }
}

fn ensure_business_success(
    code: Option<i32>,
    success: Option<bool>,
) -> Result<(), TradeGatewayError> {
    if is_business_success(code, success) {
        Ok(())
    } else {
        Err(TradeGatewayError::Unavailable)
    }
}

fn ensure_quote_business_success(
    code: Option<i32>,
    success: Option<bool>,
) -> Result<(), TradeGatewayError> {
    match code {
        Some(40_401) => Err(TradeGatewayError::QuoteExpired),
        Some(40_462) => Err(TradeGatewayError::QuoteMismatch),
        _ => ensure_business_success(code, success),
    }
}
