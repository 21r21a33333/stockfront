use std::str::FromStr;

use alloy_primitives::Address;
use binance_web3_rwa::models;
use stockfront_core::bstock_trade::{
    PreparedApproval, PreparedTrade, RfqVenue, SubmittedOrder, TokenAmount, TradeGatewayError,
    TradeOrderState, TradeOrderStatus, TradeQuote, TradeQuoteRequest,
};
use uuid::Uuid;

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
            venue: parse_venue(&venue)?,
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

pub(super) struct ProviderPreparedTrade(pub(super) models::RfqOrderPayload);

impl From<ProviderPreparedTrade> for PreparedTrade {
    fn from(value: ProviderPreparedTrade) -> Self {
        let rfq = value.0;
        Self {
            submission_id: Uuid::new_v4(),
            rfq_order_id: rfq.order_id,
            venue: domain_venue(rfq.vendor),
            typed_data: rfq.typed_data_to_sign,
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
        provider_quote.to_token_amount = Some("34".to_owned());
        provider_quote.price_impact_percent = Some(Some("0.25".to_owned()));
        provider_quote.approve_target = Some(Some(Address::with_last_byte(4).to_string()));

        let quote = TradeQuote::try_from(ProviderQuote {
            value: provider_quote,
            request: &request,
        })
        .unwrap_or_else(|error| panic!("valid provider quote rejected: {error}"));

        assert_eq!(quote.route_quote_id, "route-quote");
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
}
