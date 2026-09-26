use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::http) struct BorrowingMarketsRequest {
    pub(in crate::http) collateral_token: Option<String>,
    pub(in crate::http) debt_token: Option<String>,
}
