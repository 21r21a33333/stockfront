use std::str::FromStr;

use alloy_primitives::Address;
use binance_web3_rwa::models;
use stockfront_core::bstock_trade::{
    PreparedApproval, PreparedTrade, RfqVenue, SubmittedTrade, TradeGatewayError, TradeQuote,
    TradeState, TradeStatus,
};
use uuid::Uuid;

pub(super) struct ProviderQuote(pub(super) models::AggregatedQuote);

impl TryFrom<ProviderQuote> for TradeQuote {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderQuote) -> Result<Self, Self::Error> {
        let quote = value.0;
        let venue = required(quote.vendor_name)?;
        let raw_bstock_amount = required(quote.to_token_amount)?;
        let price_impact_percent = quote
            .price_impact_percent
            .flatten()
            .ok_or(TradeGatewayError::InvalidData)?;
        Ok(Self {
            quote_id: required(quote.quote_id)?,
            venue: parse_venue(&venue)?,
            raw_bstock_amount: parse(&raw_bstock_amount)?,
            price_impact_percent: parse(&price_impact_percent)?,
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
            calldata: parse(&value.value.data)?,
            gas_limit: parse(&value.value.gas_limit)?,
            gas_price: parse(&value.value.gas_price)?,
        })
    }
}

pub(super) struct ProviderPreparedTrade(pub(super) models::RfqOrderPayload);

impl From<ProviderPreparedTrade> for PreparedTrade {
    fn from(value: ProviderPreparedTrade) -> Self {
        let rfq = value.0;
        Self {
            request_id: Uuid::new_v4(),
            quote_id: rfq.order_id,
            venue: domain_venue(rfq.vendor),
            typed_data: rfq.typed_data_to_sign,
        }
    }
}

pub(super) struct ProviderSubmittedTrade(pub(super) models::SubmittedRfqOrder);

impl From<ProviderSubmittedTrade> for SubmittedTrade {
    fn from(value: ProviderSubmittedTrade) -> Self {
        let order = value.0;
        Self {
            order_id: order.order_id,
            state: parse_state(&order.status),
            created_at_ms: order.created_at,
        }
    }
}

pub(super) struct ProviderTradeStatus(pub(super) models::RfqOrderStatus);

impl TryFrom<ProviderTradeStatus> for TradeStatus {
    type Error = TradeGatewayError;

    fn try_from(value: ProviderTradeStatus) -> Result<Self, Self::Error> {
        let status = value.0;
        Ok(Self {
            state: parse_state(&status.status),
            transaction_hash: parse_optional(status.tx_hash.as_ref().and_then(Option::as_deref))?,
            raw_settlement_amount: parse_optional(
                status.from_amount.as_ref().and_then(Option::as_deref),
            )?,
            raw_bstock_amount: parse_optional(
                status.to_amount.as_ref().and_then(Option::as_deref),
            )?,
            created_at_ms: status.created_at,
            filled_at_ms: status.filled_at.flatten(),
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

fn parse_venue(value: &str) -> Result<RfqVenue, TradeGatewayError> {
    match value {
        "InchFusion" => Ok(RfqVenue::InchFusion),
        "CowSwap" => Ok(RfqVenue::CowSwap),
        "PcsXRfq" => Ok(RfqVenue::PcsXRfq),
        _ => Err(TradeGatewayError::InvalidData),
    }
}

fn parse_state(value: &str) -> TradeState {
    match value.trim().to_ascii_uppercase().as_str() {
        "PENDING" | "PENDING_VENDOR" | "CREATED" => TradeState::PendingVendor,
        "PENDING_ONCHAIN" | "SUBMITTED" => TradeState::PendingOnchain,
        "FILLED" | "SUCCESS" => TradeState::Filled,
        "FAILED" => TradeState::Failed,
        "EXPIRED" => TradeState::Expired,
        "CANCELLED" | "CANCELED" => TradeState::Cancelled,
        _ => TradeState::Unknown,
    }
}

fn parse<T>(value: &str) -> Result<T, TradeGatewayError>
where
    T: FromStr,
{
    value.parse().map_err(|_| TradeGatewayError::InvalidData)
}

fn parse_optional<T>(value: Option<&str>) -> Result<Option<T>, TradeGatewayError>
where
    T: FromStr,
{
    value.map(parse).transpose()
}

fn required(value: Option<String>) -> Result<String, TradeGatewayError> {
    value.ok_or(TradeGatewayError::InvalidData)
}
