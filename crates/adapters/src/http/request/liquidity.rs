use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::http) struct LiquidityOpportunitiesRequest {
    pub(in crate::http) bstock_token: String,
}
