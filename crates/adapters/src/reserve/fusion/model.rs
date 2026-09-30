use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stockfront_core::bstock_trade::TradeGatewayError;

#[derive(Debug, Deserialize)]
pub(super) struct FusionResponse<T> {
    pub(super) status: String,
    pub(super) result: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionQuote {
    pub(super) available: bool,
    pub(super) amount_out: Option<String>,
    pub(super) min_amount_out: Option<String>,
    pub(super) approval_address: Option<String>,
    pub(super) approval_needed: Option<bool>,
    pub(super) order: Option<FusionOrder>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionOrder {
    pub(super) kind: String,
    pub(super) order_hash: String,
    pub(super) quote_id: String,
    pub(super) order: FusionLimitOrder,
    pub(super) extension: String,
    pub(super) typed_data: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionLimitOrder {
    pub(super) salt: String,
    pub(super) maker: String,
    pub(super) receiver: String,
    pub(super) maker_asset: String,
    pub(super) taker_asset: String,
    pub(super) making_amount: String,
    pub(super) taking_amount: String,
    pub(super) maker_traits: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionSubmitRequest<'request> {
    pub(super) chain_id: u64,
    pub(super) order: &'request FusionLimitOrder,
    pub(super) extension: &'request str,
    pub(super) quote_id: &'request str,
    pub(super) signature: &'request str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionSubmittedOrder {
    pub(super) order_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FusionOrderStatus {
    pub(super) status: Option<String>,
    pub(super) executed_buy_amount: Option<String>,
    pub(super) tx_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum FusionContinuation {
    Quote {
        minimum_buy_amount: String,
    },
    Order {
        order_hash: String,
        order: Box<FusionLimitOrder>,
        extension: String,
    },
}

impl FusionContinuation {
    pub(super) fn encode(&self) -> Result<String, TradeGatewayError> {
        serde_json::to_vec(self)
            .map(|value| URL_SAFE_NO_PAD.encode(value))
            .map_err(|_| TradeGatewayError::InvalidData)
    }

    pub(super) fn decode(value: &str) -> Result<Self, TradeGatewayError> {
        URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| TradeGatewayError::InvalidData)
            .and_then(|value| {
                serde_json::from_slice(&value).map_err(|_| TradeGatewayError::InvalidData)
            })
    }
}
