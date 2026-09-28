use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::http) struct CarryOpportunitiesRequest {
    pub(in crate::http) collateral_token: String,
    pub(in crate::http) debt_token: String,
}
