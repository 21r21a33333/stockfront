use alloy::{sol, sol_types::SolCall};
use alloy_primitives::{Address, B256, U256};
use async_trait::async_trait;
use reqwest::{RequestBuilder, Response, StatusCode};
use rust_decimal::Decimal;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use stockfront_core::{
    bstock_market::BNB_CHAIN_ID,
    bstock_trade::{
        ApprovalRequest, PrepareTradeRequest, PreparedApproval, PreparedTrade, RfqVenue,
        SubmitOrderRequest, SubmittedOrder, TokenAmount, TradeExecutionMode, TradeGateway,
        TradeGatewayError, TradeOrderState, TradeOrderStatus, TradeQuote, TradeQuoteRequest,
        TradeTransactionStatus,
    },
};

use super::model::{
    FusionContinuation, FusionOrderStatus, FusionQuote, FusionResponse, FusionSubmitRequest,
    FusionSubmittedOrder,
};
use crate::reserve::catalog::ReserveAdapter;

const VENUE: &str = "InchFusion";
const REQUOTE_TOLERANCE_BPS: u64 = 10;
const FUSION_QUOTE_PATH: &str = "1inch/fusion/quote";
const FUSION_ORDER_PATH: &str = "1inch/fusion/order";
const FUSION_SPENDER: Address =
    alloy_primitives::address!("111111125421ca6dc452d289314280a0f8842a65");

sol! {
    interface IERC20Approval {
        function approve(address spender, uint256 amount) external returns (bool);
    }
}

#[async_trait]
impl TradeGateway for ReserveAdapter {
    async fn quotes(
        &self,
        request: &TradeQuoteRequest,
    ) -> Result<Vec<TradeQuote>, TradeGatewayError> {
        let quote = self.quote(request).await?;
        let buy_amount = parse_amount(quote.amount_out.as_deref())?;
        let minimum_buy_amount = required(quote.min_amount_out)?;
        let approval_target = if quote.approval_needed.unwrap_or(false) {
            parse_optional(quote.approval_address)?
        } else {
            None
        };

        Ok(vec![TradeQuote {
            route_quote_id: FusionContinuation::Quote { minimum_buy_amount }.encode()?,
            venue: VENUE.to_owned(),
            execution_mode: TradeExecutionMode::Rfq,
            sell: TokenAmount {
                token: request.sell_token,
                amount: request.sell_amount,
            },
            buy: TokenAmount {
                token: request.buy_token,
                amount: buy_amount,
            },
            price_impact_percent: None,
            approval_target,
        }])
    }

    async fn prepare_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<PreparedApproval, TradeGatewayError> {
        if request.venue != VENUE {
            return Err(TradeGatewayError::Rejected);
        }
        Ok(PreparedApproval {
            to: request.token,
            spender: FUSION_SPENDER,
            data: IERC20Approval::approveCall {
                spender: FUSION_SPENDER,
                amount: request.amount,
            }
            .abi_encode()
            .into(),
            gas_limit: None,
            gas_price: None,
        })
    }

    async fn prepare_trade(
        &self,
        request: &PrepareTradeRequest,
        _slippage_percent: Decimal,
    ) -> Result<PreparedTrade, TradeGatewayError> {
        let FusionContinuation::Quote { minimum_buy_amount } =
            FusionContinuation::decode(&request.route_quote_id)?
        else {
            return Err(TradeGatewayError::InvalidData);
        };
        let displayed_minimum = minimum_buy_amount
            .parse::<U256>()
            .map_err(|_| TradeGatewayError::InvalidData)?;
        let quote = self.quote(&request.trade).await?;
        let refreshed_minimum = parse_amount(quote.min_amount_out.as_deref())?;
        if !accepts_requote(displayed_minimum, refreshed_minimum)? {
            return Err(TradeGatewayError::QuoteMismatch);
        }

        let order = quote.order.ok_or(TradeGatewayError::InvalidData)?;
        if order.kind != "signature" {
            return Err(TradeGatewayError::Rejected);
        }
        let typed_data = order.typed_data.ok_or(TradeGatewayError::InvalidData)?;
        let submission_id = order
            .quote_id
            .parse()
            .map_err(|_| TradeGatewayError::InvalidData)?;
        let route_quote_id = FusionContinuation::Order {
            order_hash: order.order_hash,
            order: Box::new(order.order),
            extension: order.extension,
        }
        .encode()?;

        Ok(PreparedTrade::Rfq {
            submission_id,
            route_quote_id,
            venue: RfqVenue::InchFusion,
            typed_data: serde_json::to_string(&typed_data)
                .map_err(|_| TradeGatewayError::InvalidData)?,
        })
    }

    async fn submit_order(
        &self,
        request: &SubmitOrderRequest,
    ) -> Result<SubmittedOrder, TradeGatewayError> {
        if !matches!(request.venue, RfqVenue::InchFusion) {
            return Err(TradeGatewayError::Rejected);
        }
        let FusionContinuation::Order {
            order_hash,
            order,
            extension,
        } = FusionContinuation::decode(&request.route_quote_id)?
        else {
            return Err(TradeGatewayError::InvalidData);
        };
        let body = FusionSubmitRequest {
            chain_id: BNB_CHAIN_ID,
            order: &order,
            extension: &extension,
            quote_id: &request.submission_id.to_string(),
            signature: &request.signature,
        };
        let response: FusionResponse<FusionSubmittedOrder> = execute(
            self.client
                .post(FUSION_ORDER_PATH)
                .map_err(|_| TradeGatewayError::InvalidData)?
                .json(&body),
            TradeGatewayError::Rejected,
        )
        .await?;
        let submitted_hash = successful(response)?
            .order_hash
            .ok_or(TradeGatewayError::InvalidData)?;
        if submitted_hash != order_hash {
            return Err(TradeGatewayError::InvalidData);
        }

        Ok(SubmittedOrder {
            order_id: order_hash,
            state: TradeOrderState::PendingVendor,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn order_status(&self, order_id: &str) -> Result<TradeOrderStatus, TradeGatewayError> {
        let path = format!("{FUSION_ORDER_PATH}/{BNB_CHAIN_ID}/{order_id}");
        let response = send(
            self.client
                .get(&path)
                .map_err(|_| TradeGatewayError::InvalidData)?,
        )
        .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(TradeGatewayError::OrderNotFound);
        }
        let response: FusionResponse<FusionOrderStatus> =
            decode(response, TradeGatewayError::Unavailable).await?;
        let status = successful(response)?;
        Ok(TradeOrderStatus {
            state: order_state(status.status.as_deref()),
            transaction_hash: parse_optional(status.tx_hash)?,
            filled_sell_amount: None,
            filled_buy_amount: parse_optional(status.executed_buy_amount)?,
            created_at_ms: None,
            filled_at_ms: None,
        })
    }

    async fn transaction_status(
        &self,
        _transaction_hash: B256,
    ) -> Result<TradeTransactionStatus, TradeGatewayError> {
        Err(TradeGatewayError::TransactionNotFound)
    }
}

impl ReserveAdapter {
    async fn quote(&self, request: &TradeQuoteRequest) -> Result<FusionQuote, TradeGatewayError> {
        let response: FusionResponse<FusionQuote> = execute(
            self.client
                .get(FUSION_QUOTE_PATH)
                .map_err(|_| TradeGatewayError::InvalidData)?
                .query(&[
                    ("chainId", BNB_CHAIN_ID.to_string()),
                    ("tokenIn", request.sell_token.to_string()),
                    ("tokenOut", request.buy_token.to_string()),
                    ("amountIn", request.sell_amount.to_string()),
                    (
                        "slippage",
                        self.slippage_denominator.normalize().to_string(),
                    ),
                    ("signer", request.wallet.as_address().to_string()),
                ]),
            TradeGatewayError::Unavailable,
        )
        .await?;
        let quote = successful(response)?;
        if quote.available {
            Ok(quote)
        } else {
            Err(TradeGatewayError::Rejected)
        }
    }
}

fn successful<T>(response: FusionResponse<T>) -> Result<T, TradeGatewayError> {
    if response.status != "success" {
        return Err(TradeGatewayError::Rejected);
    }
    response.result.ok_or(TradeGatewayError::InvalidData)
}

fn required(value: Option<String>) -> Result<String, TradeGatewayError> {
    value.ok_or(TradeGatewayError::InvalidData)
}

fn parse_amount(value: Option<&str>) -> Result<U256, TradeGatewayError> {
    value
        .ok_or(TradeGatewayError::InvalidData)?
        .parse()
        .map_err(|_| TradeGatewayError::InvalidData)
}

fn parse_optional<T>(value: Option<String>) -> Result<Option<T>, TradeGatewayError>
where
    T: FromStr,
{
    value
        .map(|value| value.parse().map_err(|_| TradeGatewayError::InvalidData))
        .transpose()
}

async fn execute<T>(
    request: RequestBuilder,
    http_error: TradeGatewayError,
) -> Result<T, TradeGatewayError>
where
    T: DeserializeOwned,
{
    decode(send(request).await?, http_error).await
}

async fn send(request: RequestBuilder) -> Result<Response, TradeGatewayError> {
    request
        .send()
        .await
        .map_err(|_| TradeGatewayError::Unavailable)
}

async fn decode<T>(
    response: Response,
    http_error: TradeGatewayError,
) -> Result<T, TradeGatewayError>
where
    T: DeserializeOwned,
{
    response
        .error_for_status()
        .map_err(|_| http_error)?
        .json()
        .await
        .map_err(|_| TradeGatewayError::InvalidData)
}

fn order_state(value: Option<&str>) -> TradeOrderState {
    match value {
        Some("fulfilled") => TradeOrderState::Filled,
        Some("expired") => TradeOrderState::Expired,
        Some("cancelled") => TradeOrderState::Cancelled,
        Some("open" | "pending") => TradeOrderState::PendingVendor,
        _ => TradeOrderState::Unknown,
    }
}

fn accepts_requote(displayed: U256, refreshed: U256) -> Result<bool, TradeGatewayError> {
    let accepted = displayed
        .checked_mul(U256::from(10_000 - REQUOTE_TOLERANCE_BPS))
        .ok_or(TradeGatewayError::InvalidData)?
        / U256::from(10_000);
    Ok(refreshed >= accepted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requote_accepts_exact_tolerance_and_rejects_one_unit_below() {
        let displayed = U256::from(10_000);

        assert!(matches!(
            accepts_requote(displayed, U256::from(9_990)),
            Ok(true)
        ));
        assert!(matches!(
            accepts_requote(displayed, U256::from(9_989)),
            Ok(false)
        ));
    }
}
