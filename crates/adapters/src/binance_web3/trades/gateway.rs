use alloy_primitives::B256;
use async_trait::async_trait;
use binance_web3_rwa::{
    apis::trading_api::{
        BuildSwapTransactionParams, GetAggregatedQuoteParams, GetErc20ApproveTransactionParams,
        GetRfqOrderStatusParams, GetTransactionStatusParams, SubmitRfqOrderParams,
        build_swap_transaction, get_aggregated_quote, get_erc20_approve_transaction,
        get_rfq_order_status, get_transaction_status, submit_rfq_order,
    },
    models,
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_market::BNB_CHAIN_ID,
    bstock_trade::{
        ApprovalRequest, BstockTradeGateway, PrepareTradeRequest, PreparedApproval, PreparedTrade,
        SubmitOrderRequest, SubmittedOrder, TradeGatewayError, TradeOrderStatus, TradeQuote,
        TradeQuoteRequest, TradeTransactionStatus,
    },
};

use super::{
    super::client::{BinanceWeb3Client, is_business_success},
    mapping::{
        ProviderApproval, ProviderOrderStatus, ProviderPreparedTrade, ProviderQuote,
        ProviderSubmittedOrder, ProviderTransactionStatus, provider_venue,
    },
};

#[async_trait]
impl BstockTradeGateway for BinanceWeb3Client {
    async fn quotes(
        &self,
        request: &TradeQuoteRequest,
    ) -> Result<Vec<TradeQuote>, TradeGatewayError> {
        let response = get_aggregated_quote(
            &self.configuration,
            GetAggregatedQuoteParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                amount: request.sell_amount.to_string(),
                from_token_address: request.sell_token.to_string(),
                to_token_address: request.buy_token.to_string(),
                user_wallet_address: request.wallet.as_address().to_string(),
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_business_success(response.code, response.success)?;
        Ok(response
            .data
            .unwrap_or_default()
            .into_iter()
            .filter_map(|quote| {
                TradeQuote::try_from(ProviderQuote {
                    value: quote,
                    request,
                })
                .ok()
            })
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
                token_contract_address: request.token.to_string(),
                approve_amount: request.amount.to_string(),
                vendor: request.venue.clone(),
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_business_success(response.code, response.success)?;
        let approval = response
            .data
            .unwrap_or_default()
            .into_iter()
            .next()
            .ok_or(TradeGatewayError::InvalidData)?;
        PreparedApproval::try_from(ProviderApproval {
            token: request.token,
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
                amount: request.trade.sell_amount.to_string(),
                from_token_address: request.trade.sell_token.to_string(),
                to_token_address: request.trade.buy_token.to_string(),
                user_wallet_address: request.trade.wallet.as_address().to_string(),
                quote_id: request.route_quote_id.clone(),
                slippage_percent: slippage_percent.to_string(),
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_quote_business_success(response.code, response.success)?;
        let prepared = response.data.ok_or(TradeGatewayError::InvalidData)?;
        PreparedTrade::try_from(ProviderPreparedTrade {
            value: *prepared,
            route_quote_id: request.route_quote_id.clone(),
        })
    }

    async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, TradeGatewayError> {
        let mut body = models::SubmitRfqOrderRequest::new(
            request.submission_id,
            request.signature.clone(),
            provider_venue(request.venue),
            request.route_quote_id.clone(),
        );
        body.signing_scheme = Some(models::RfqSigningScheme::Eip712);
        let response = submit_rfq_order(
            &self.configuration,
            SubmitRfqOrderParams {
                submit_rfq_order_request: body,
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_quote_business_success(response.code, response.success)?;
        let submitted = response.data.ok_or(TradeGatewayError::InvalidData)?;
        Ok(ProviderSubmittedOrder(*submitted).into())
    }

    async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, TradeGatewayError> {
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
            error => map_provider_error(error),
        })?;
        if matches!(response.code, Some(40_363 | 404 | 40_404)) {
            return Err(TradeGatewayError::OrderNotFound);
        }
        ensure_business_success(response.code, response.success)?;
        let status = response.data.ok_or(TradeGatewayError::InvalidData)?;
        TradeOrderStatus::try_from(ProviderOrderStatus(*status))
    }

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<TradeTransactionStatus, TradeGatewayError> {
        let response = get_transaction_status(
            &self.configuration,
            GetTransactionStatusParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                tx_hash: transaction_hash.to_string(),
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_business_success(response.code, response.success)?;
        let status = response
            .data
            .ok_or(TradeGatewayError::TransactionNotFound)?;
        TradeTransactionStatus::try_from(ProviderTransactionStatus(*status))
    }
}

fn ensure_business_success(
    code: Option<i32>,
    success: Option<bool>,
) -> Result<(), TradeGatewayError> {
    if is_business_success(code, success) {
        Ok(())
    } else {
        match code {
            Some(40_000..=49_999) if !matches!(code, Some(40_101..=40_104 | 42_900)) => {
                Err(TradeGatewayError::Rejected)
            }
            _ => Err(TradeGatewayError::Unavailable),
        }
    }
}

fn map_provider_error<T>(error: binance_web3_rwa::apis::Error<T>) -> TradeGatewayError {
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
            TradeGatewayError::Rejected
        }
        _ => TradeGatewayError::Unavailable,
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
