use alloy_primitives::Address;
use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::bstock_trade::{
    PreparedApproval, PreparedTrade, PreparedTransaction, RfqVenue, SettledTokenAmount,
    SubmittedOrder, TokenAmount, TradeExecutionMode, TradeGatewayError, TradeOrderState,
    TradeOrderStatus, TradeQuote, TradeQuoteRequest, TradeTransactionState, TradeTransactionStatus,
};
use uuid::Uuid;

use super::super::provider::{parse, parse_optional, required};

pub(super) struct ProviderQuote<'request> {
    pub(super) value: models::AggregatedQuote,
    pub(super) request: &'request TradeQuoteRequest,
}

impl TryFrom<ProviderQuote<'_>> for TradeQuote {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderQuote) -> Result<Self, Self::Error> {
        let quote = value.value;
        let venue = required(quote.vendor_name)?;
        let buy_amount = required(quote.to_token_amount)?;
        let price_impact_percent = quote
            .price_impact_percent
            .flatten()
            .ok_or(TradeGatewayError::InvalidData)?;
        Ok(Self {
            route_quote_id: required(quote.quote_id)?,
            venue,
            execution_mode: parse_execution_mode(&required(quote.execution_mode)?)?,
            sell: TokenAmount {
                token: value.request.sell_token,
                amount: value.request.sell_amount,
            },
            buy: TokenAmount {
                token: value.request.buy_token,
                amount: parse(&buy_amount)?,
            },
            price_impact_percent: parse(&price_impact_percent)?,
            approval_target: parse_optional(
                quote.approve_target.as_ref().and_then(Option::as_deref),
            )?,
        })
    }
}

pub(super) struct ProviderApproval {
    pub(super) token: Address,
    pub(super) value: models::ApproveTransaction,
}

impl TryFrom<ProviderApproval> for PreparedApproval {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderApproval) -> Result<Self, Self::Error> {
        Ok(Self {
            to: value.token,
            spender: parse(&value.value.dex_contract_address)?,
            data: parse(&value.value.data)?,
            gas_limit: parse(&value.value.gas_limit)?,
            gas_price: parse(&value.value.gas_price)?,
        })
    }
}

pub(super) struct ProviderPreparedTrade {
    pub(super) value: models::BuildSwapTransactionData,
    pub(super) route_quote_id: String,
}

impl TryFrom<ProviderPreparedTrade> for PreparedTrade {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderPreparedTrade) -> Result<Self, Self::Error> {
        match value.value.execution_mode.as_str() {
            "SWAP" => {
                let transaction = value.value.tx.ok_or(TradeGatewayError::InvalidData)?;
                Ok(Self::Swap {
                    transaction: PreparedTransaction {
                        to: parse(&transaction.to)?,
                        data: parse(&transaction.data)?,
                        value: parse(&transaction.value)?,
                        gas_limit: parse(&transaction.gas)?,
                        gas_price: parse(&transaction.gas_price)?,
                        max_priority_fee_per_gas: parse_optional(
                            transaction.max_priority_fee_per_gas.as_deref(),
                        )?,
                        min_receive_amount: parse(&transaction.min_receive_amount)?,
                    },
                })
            }
            "RFQ" => {
                let rfq = value.value.rfq.ok_or(TradeGatewayError::InvalidData)?;
                Ok(Self::Rfq {
                    submission_id: Uuid::new_v4(),
                    route_quote_id: value.route_quote_id,
                    venue: domain_venue(rfq.vendor),
                    typed_data: rfq.typed_data_to_sign,
                })
            }
            _ => Err(TradeGatewayError::InvalidData),
        }
    }
}

pub(super) struct ProviderSubmittedOrder(pub(super) models::SubmittedRfqOrder);

impl From<ProviderSubmittedOrder> for SubmittedOrder {
    fn from(value: ProviderSubmittedOrder) -> Self {
        let order = value.0;
        Self {
            order_id: order.order_id,
            state: parse_state(&order.status),
            created_at_ms: order.created_at,
        }
    }
}

pub(super) struct ProviderOrderStatus(pub(super) models::RfqOrderStatus);

impl TryFrom<ProviderOrderStatus> for TradeOrderStatus {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderOrderStatus) -> Result<Self, Self::Error> {
        let status = value.0;
        Ok(Self {
            state: parse_state(&status.status),
            transaction_hash: parse_optional(status.tx_hash.as_ref().and_then(Option::as_deref))?,
            filled_sell_amount: parse_optional(
                status.from_amount.as_ref().and_then(Option::as_deref),
            )?,
            filled_buy_amount: parse_optional(
                status.to_amount.as_ref().and_then(Option::as_deref),
            )?,
            created_at_ms: status.created_at,
            filled_at_ms: status.filled_at.flatten(),
        })
    }
}

pub(super) struct ProviderTransactionStatus(pub(super) models::TransactionStatus);

impl TryFrom<ProviderTransactionStatus> for TradeTransactionStatus {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderTransactionStatus) -> Result<Self, Self::Error> {
        let status = value.0;
        Ok(Self {
            state: parse_transaction_state(&required(status.status)?),
            sold: parse_token_amounts(status.from_token_details.unwrap_or_default())?,
            bought: parse_token_amounts(status.to_token_details.unwrap_or_default())?,
            block_number: parse_optional(status.height.as_deref())?,
            executed_at_ms: parse_optional_seconds_as_millis(status.tx_time.as_deref())?,
            error: status.error_msg,
        })
    }
}

pub(super) fn provider_venue(value: RfqVenue) -> models::RfqVenue {
    match value {
        RfqVenue::InchFusion => models::RfqVenue::InchFusion,
        RfqVenue::CowSwap => models::RfqVenue::CowSwap,
        RfqVenue::PcsXRfq => models::RfqVenue::PcsXRfq,
    }
}

fn domain_venue(value: models::RfqVenue) -> RfqVenue {
    match value {
        models::RfqVenue::InchFusion => RfqVenue::InchFusion,
        models::RfqVenue::CowSwap => RfqVenue::CowSwap,
        models::RfqVenue::PcsXRfq => RfqVenue::PcsXRfq,
    }
}

fn parse_execution_mode(value: &str) -> Result<TradeExecutionMode, TradeGatewayError> {
    match value {
        "SWAP" => Ok(TradeExecutionMode::Swap),
        "RFQ" => Ok(TradeExecutionMode::Rfq),
        _ => Err(TradeGatewayError::InvalidData),
    }
}

fn parse_state(value: &str) -> TradeOrderState {
    match value.trim().to_ascii_uppercase().as_str() {
        "PENDING" | "PENDING_VENDOR" | "CREATED" => TradeOrderState::PendingVendor,
        "PENDING_ONCHAIN" | "SUBMITTED" => TradeOrderState::PendingOnchain,
        "FILLED" | "SUCCESS" => TradeOrderState::Filled,
        "FAILED" => TradeOrderState::Failed,
        "EXPIRED" => TradeOrderState::Expired,
        "CANCELLED" | "CANCELED" => TradeOrderState::Cancelled,
        _ => TradeOrderState::Unknown,
    }
}

fn parse_transaction_state(value: &str) -> TradeTransactionState {
    match value.trim().to_ascii_uppercase().as_str() {
        "PENDING" => TradeTransactionState::Pending,
        "SUCCESS" => TradeTransactionState::Succeeded,
        "FAILED" => TradeTransactionState::Failed,
        _ => TradeTransactionState::Unknown,
    }
}

fn parse_token_amounts(
    values: Vec<models::TransactionTokenAmount>,
) -> Result<Vec<SettledTokenAmount>, TradeGatewayError> {
    values
        .into_iter()
        .map(|value| {
            Ok(SettledTokenAmount {
                token: parse(&required(value.token_address)?)?,
                amount: parse_decimal(&required(value.amount)?)?,
            })
        })
        .collect()
}

fn parse_decimal(value: &str) -> Result<Decimal, TradeGatewayError> {
    let normalized = if value.contains('.') {
        value.trim_end_matches('0').trim_end_matches('.')
    } else {
        value
    };
    Ok(parse(normalized)?)
}

fn parse_optional_seconds_as_millis(value: Option<&str>) -> Result<Option<i64>, TradeGatewayError> {
    value
        .map(|value| {
            parse::<i64>(value)?
                .checked_mul(1_000)
                .ok_or(TradeGatewayError::InvalidData)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use stockfront_core::{
        bstock_trade::{TradeOrderState, TradeQuote, TradeQuoteRequest},
        wallet_portfolio::WalletAddress,
    };

    use super::*;

    #[test]
    fn quote_keeps_sell_and_buy_amounts_direction_neutral() {
        let sell_token = Address::with_last_byte(1);
        let buy_token = Address::with_last_byte(2);
        let request = TradeQuoteRequest {
            wallet: WalletAddress::try_from(Address::with_last_byte(3))
                .unwrap_or_else(|error| panic!("valid wallet rejected: {error}")),
            sell_token,
            buy_token,
            sell_amount: U256::from(12),
        };
        let mut provider_quote = models::AggregatedQuote::new();
        provider_quote.quote_id = Some("route-quote".to_owned());
        provider_quote.vendor_name = Some("CowSwap".to_owned());
        provider_quote.execution_mode = Some("RFQ".to_owned());
        provider_quote.to_token_amount = Some("34".to_owned());
        provider_quote.price_impact_percent = Some(Some("0.25".to_owned()));
        provider_quote.approve_target = Some(Some(Address::with_last_byte(4).to_string()));

        let quote = TradeQuote::try_from(ProviderQuote {
            value: provider_quote,
            request: &request,
        })
        .unwrap_or_else(|error| panic!("valid provider quote rejected: {error}"));

        assert_eq!(quote.route_quote_id, "route-quote");
        assert_eq!(quote.venue, "CowSwap");
        assert!(matches!(quote.execution_mode, TradeExecutionMode::Rfq));
        assert_eq!(quote.sell.token, sell_token);
        assert_eq!(quote.sell.amount, U256::from(12));
        assert_eq!(quote.buy.token, buy_token);
        assert_eq!(quote.buy.amount, U256::from(34));
        assert_eq!(quote.approval_target, Some(Address::with_last_byte(4)));
    }

    #[test]
    fn filled_status_uses_sell_and_buy_semantics() {
        let mut provider_status = models::RfqOrderStatus::new("FILLED".to_owned(), 10);
        provider_status.from_amount = Some(Some("12".to_owned()));
        provider_status.to_amount = Some(Some("34".to_owned()));
        provider_status.filled_at = Some(Some(20));

        let status = TradeOrderStatus::try_from(ProviderOrderStatus(provider_status))
            .unwrap_or_else(|error| panic!("valid provider status rejected: {error}"));

        assert!(matches!(status.state, TradeOrderState::Filled));
        assert_eq!(status.filled_sell_amount, Some(U256::from(12)));
        assert_eq!(status.filled_buy_amount, Some(U256::from(34)));
        assert_eq!(status.filled_at_ms, Some(20));
    }

    #[test]
    fn prepared_swap_keeps_the_wallet_transaction_payload() {
        let to = Address::with_last_byte(5);
        let mut transaction = models::SwapTransactionPayload::new(
            to.to_string(),
            "0x1234".to_owned(),
            "0".to_owned(),
            "450000".to_owned(),
            "3000000000".to_owned(),
            "99".to_owned(),
        );
        transaction.max_priority_fee_per_gas = Some("1000000000".to_owned());
        let mut prepared = models::BuildSwapTransactionData::new("SWAP".to_owned());
        prepared.tx = Some(Box::new(transaction));

        let trade = PreparedTrade::try_from(ProviderPreparedTrade {
            value: prepared,
            route_quote_id: "unused-for-swap".to_owned(),
        })
        .unwrap_or_else(|error| panic!("valid SWAP payload rejected: {error}"));

        let PreparedTrade::Swap { transaction } = trade else {
            panic!("SWAP payload mapped to RFQ");
        };
        assert_eq!(transaction.to, to);
        assert_eq!(transaction.data.as_ref(), &[0x12, 0x34]);
        assert_eq!(transaction.gas_limit, U256::from(450_000));
        assert_eq!(transaction.min_receive_amount, U256::from(99));
    }

    #[test]
    fn prepared_rfq_keeps_the_quote_for_order_submission() {
        let rfq = models::RfqOrderPayload::new(models::RfqVenue::CowSwap, "typed-data".to_owned());
        let mut prepared = models::BuildSwapTransactionData::new("RFQ".to_owned());
        prepared.rfq = Some(Box::new(rfq));

        let trade = PreparedTrade::try_from(ProviderPreparedTrade {
            value: prepared,
            route_quote_id: "route-quote".to_owned(),
        })
        .unwrap_or_else(|error| panic!("valid RFQ payload rejected: {error}"));

        let PreparedTrade::Rfq {
            route_quote_id,
            venue,
            typed_data,
            ..
        } = trade
        else {
            panic!("RFQ payload mapped to SWAP");
        };
        assert_eq!(route_quote_id, "route-quote");
        assert!(matches!(venue, RfqVenue::CowSwap));
        assert_eq!(typed_data, "typed-data");
    }

    #[test]
    fn transaction_status_keeps_binance_swap_evidence() {
        let sold_token = Address::with_last_byte(6);
        let bought_token = Address::with_last_byte(7);
        let mut sold = models::TransactionTokenAmount::new();
        sold.token_address = Some(sold_token.to_string());
        sold.amount =
            Some("12.250000000000000000000000000000000000000000000000000000000000".to_owned());
        let mut bought = models::TransactionTokenAmount::new();
        bought.token_address = Some(bought_token.to_string());
        bought.amount = Some("34.5".to_owned());
        let mut provider_status = models::TransactionStatus::new();
        provider_status.status = Some("success".to_owned());
        provider_status.height = Some("123".to_owned());
        provider_status.tx_time = Some("456".to_owned());
        provider_status.from_token_details = Some(vec![sold]);
        provider_status.to_token_details = Some(vec![bought]);

        let status = TradeTransactionStatus::try_from(ProviderTransactionStatus(provider_status))
            .unwrap_or_else(|error| panic!("valid transaction status rejected: {error}"));

        assert!(matches!(status.state, TradeTransactionState::Succeeded));
        assert_eq!(status.sold[0].token, sold_token);
        assert_eq!(status.sold[0].amount, Decimal::new(1_225, 2));
        assert_eq!(status.bought[0].token, bought_token);
        assert_eq!(status.bought[0].amount, Decimal::new(345, 1));
        assert_eq!(status.block_number, Some(123));
        assert_eq!(status.executed_at_ms, Some(456_000));
    }
}
