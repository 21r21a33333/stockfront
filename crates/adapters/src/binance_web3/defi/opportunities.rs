use std::collections::HashSet;

use alloy_primitives::Address;
use async_trait::async_trait;
use binance_web3_rwa::{
    apis::de_fi_data_api::{
        GetDefiInvestmentDetailParams, ListDefiInvestmentsParams, get_defi_investment_detail,
        list_defi_investments,
    },
    models,
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingToken,
    bstock_market::BNB_CHAIN_ID,
    carry::{EarnOpportunity, EarnOpportunityProviderError, EarnOpportunityReader, EarnRateType},
};

use super::super::{
    client::BinanceWeb3Client,
    provider::{
        BinanceRequestError, InvalidProviderData, ensure_business_success, map_request_error,
        parse, required, required_non_empty,
    },
};

const PAGE_SIZE: i32 = 100;

#[async_trait]
impl EarnOpportunityReader for BinanceWeb3Client {
    async fn opportunities(
        &self,
        asset: Address,
    ) -> Result<Vec<EarnOpportunity>, EarnOpportunityProviderError> {
        let investment_ids = self.earn_investment_ids(asset).await?;
        if investment_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut opportunities = Vec::with_capacity(investment_ids.len());

        for investment_id in investment_ids {
            let Some(detail) = self.earn_investment_detail(&investment_id).await? else {
                continue;
            };
            if let Ok(opportunity) = EarnOpportunity::try_from(ProviderEarnOpportunity {
                investment_id,
                asset,
                observed_at_ms: detail.observed_at_ms,
                detail: detail.investment,
            }) {
                opportunities.push(opportunity);
            }
        }

        if opportunities.is_empty() {
            Err(EarnOpportunityProviderError::InvalidData)
        } else {
            Ok(opportunities)
        }
    }
}

impl BinanceWeb3Client {
    async fn earn_investment_ids(
        &self,
        asset: Address,
    ) -> Result<Vec<String>, EarnOpportunityProviderError> {
        let mut page_number = 1;
        let mut investment_ids = Vec::new();
        let mut seen = HashSet::new();
        let mut provider_record_count = 0_usize;

        loop {
            let response = list_defi_investments(
                &self.configuration,
                ListDefiInvestmentsParams {
                    list_defi_investments_request: earn_list_request(asset, page_number),
                },
            )
            .await
            .map_err(map_request_error)?;
            ensure_business_success(response.code, response.success)?;
            let page = required(response.data)?;
            let total = page.total;
            let investments = page.list.unwrap_or_default();
            let record_count = investments.len();
            provider_record_count += record_count;
            append_investment_ids(&mut investment_ids, &mut seen, investments);
            if !has_next_page(page_number, record_count, total) {
                break;
            }
            page_number += 1;
        }

        if provider_record_count > 0 && investment_ids.is_empty() {
            Err(EarnOpportunityProviderError::InvalidData)
        } else {
            Ok(investment_ids)
        }
    }

    async fn earn_investment_detail(
        &self,
        investment_id: &str,
    ) -> Result<Option<ProviderInvestmentDetail>, EarnOpportunityProviderError> {
        let body = models::GetDefiInvestmentDetailRequest::new(investment_id.to_owned());
        let response = get_defi_investment_detail(
            &self.configuration,
            GetDefiInvestmentDetailParams {
                get_defi_investment_detail_request: body,
            },
        )
        .await
        .map_err(map_request_error)?;
        match ensure_business_success(response.code, response.success) {
            Ok(()) => {}
            Err(BinanceRequestError::Rejected) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        Ok(Some(ProviderInvestmentDetail {
            investment: *required(response.data)?,
            observed_at_ms: required(response.timestamp)?,
        }))
    }
}

fn earn_list_request(asset: Address, page: i32) -> models::ListDefiInvestmentsRequest {
    let mut request = models::ListDefiInvestmentsRequest::new(
        models::list_defi_investments_request::InvestType::Earn,
    );
    request.token_address_list = Some(vec![asset.to_string()]);
    request.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
    request.sort_field = Some(models::list_defi_investments_request::SortField::Apy);
    request.sort_direction = Some(models::list_defi_investments_request::SortDirection::Desc);
    request.page = Some(page);
    request.size = Some(PAGE_SIZE);
    request
}

struct ProviderInvestmentDetail {
    investment: models::DefiInvestmentDetail,
    observed_at_ms: i64,
}

struct ProviderEarnOpportunity {
    investment_id: String,
    asset: Address,
    observed_at_ms: i64,
    detail: models::DefiInvestmentDetail,
}

impl TryFrom<ProviderEarnOpportunity> for EarnOpportunity {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderEarnOpportunity) -> Result<Self, Self::Error> {
        let asset_token = provider
            .detail
            .asset_token_list
            .unwrap_or_default()
            .into_iter()
            .find(|token| {
                token
                    .token_address
                    .as_deref()
                    .and_then(|address| address.parse::<Address>().ok())
                    == Some(provider.asset)
            })
            .ok_or(InvalidProviderData)?;
        let rate_type = match required_non_empty(provider.detail.apy_type)?.as_str() {
            "APY" => EarnRateType::Apy,
            "APR" => EarnRateType::Apr,
            _ => return Err(InvalidProviderData),
        };

        Ok(Self {
            investment_id: provider.investment_id,
            investment_name: required_non_empty(provider.detail.investment_name)?,
            protocol_id: required_non_empty(provider.detail.defi_protocol_id)?,
            protocol_name: required_non_empty(provider.detail.protocol_name)?,
            protocol_logo_url: provider.detail.protocol_logo.filter(|url| !url.is_empty()),
            asset: BorrowingToken {
                contract: provider.asset,
                symbol: required_non_empty(asset_token.token_symbol)?,
            },
            rate_type,
            base_rate_bps: required(provider.detail.apy_bps)?,
            base_rate_display: required_non_empty(provider.detail.apy_display)?,
            reported_tvl_usd: provider
                .detail
                .tvl
                .as_deref()
                .and_then(|value| parse::<Decimal>(value).ok()),
            investable: required(provider.detail.investable)?,
            observed_at_ms: provider.observed_at_ms,
        })
    }
}

fn append_investment_ids(
    investment_ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
    investments: Vec<models::DefiInvestment>,
) {
    for investment_id in investments
        .into_iter()
        .filter_map(|investment| investment.investment_id)
        .filter(|investment_id| !investment_id.is_empty())
    {
        if seen.insert(investment_id.clone()) {
            investment_ids.push(investment_id);
        }
    }
}

fn has_next_page(page: i32, record_count: usize, total: Option<i64>) -> bool {
    if let Some(total) = total {
        return i64::from(page) * i64::from(PAGE_SIZE) < total;
    }
    record_count == PAGE_SIZE as usize
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn maps_apy_and_optional_fields_without_float_conversion() {
        let asset = address!("55d398326f99059fF775485246999027B3197955");
        let mut detail = models::DefiInvestmentDetail::new();
        detail.defi_protocol_id = Some("venus".to_owned());
        detail.protocol_name = Some("Venus".to_owned());
        detail.investment_name = Some("USDT Supply".to_owned());
        detail.investable = Some(true);
        detail.apy_bps = Some(725);
        detail.apy_display = Some("7.25%".to_owned());
        detail.apy_type = Some("APY".to_owned());
        detail.tvl = Some("1234.56".to_owned());
        let mut token = models::DefiInvestmentToken::new();
        token.token_address = Some(asset.to_string());
        token.token_symbol = Some("USDT".to_owned());
        detail.asset_token_list = Some(vec![token]);

        let opportunity = EarnOpportunity::try_from(ProviderEarnOpportunity {
            investment_id: "investment".to_owned(),
            asset,
            observed_at_ms: 123,
            detail,
        })
        .unwrap_or_else(|_| panic!("valid fixture must map"));

        assert_eq!(opportunity.rate_type, EarnRateType::Apy);
        assert_eq!(opportunity.base_rate_bps, 725);
        assert_eq!(opportunity.base_rate_display, "7.25%");
        assert_eq!(opportunity.reported_tvl_usd, Some(Decimal::new(123_456, 2)));
    }

    #[test]
    fn supports_apr_and_omits_invalid_optional_tvl() {
        let asset = address!("55d398326f99059fF775485246999027B3197955");
        let mut detail = models::DefiInvestmentDetail::new();
        detail.defi_protocol_id = Some("lista".to_owned());
        detail.protocol_name = Some("Lista DAO".to_owned());
        detail.investment_name = Some("USDT Vault".to_owned());
        detail.investable = Some(true);
        detail.apy_bps = Some(500);
        detail.apy_display = Some("5.00%".to_owned());
        detail.apy_type = Some("APR".to_owned());
        detail.tvl = Some("unknown".to_owned());
        let mut token = models::DefiInvestmentToken::new();
        token.token_address = Some(asset.to_string());
        token.token_symbol = Some("USDT".to_owned());
        detail.asset_token_list = Some(vec![token]);

        let opportunity = EarnOpportunity::try_from(ProviderEarnOpportunity {
            investment_id: "investment".to_owned(),
            asset,
            observed_at_ms: 123,
            detail,
        })
        .unwrap_or_else(|_| panic!("valid fixture must map"));

        assert_eq!(opportunity.rate_type, EarnRateType::Apr);
        assert_eq!(opportunity.reported_tvl_usd, None);
    }

    #[test]
    fn deduplicates_investment_ids_and_tracks_provider_pages() {
        let investments = ["first", "first", "second"]
            .into_iter()
            .map(|id| {
                let mut investment = models::DefiInvestment::new();
                investment.investment_id = Some(id.to_owned());
                investment
            })
            .collect();
        let mut investment_ids = Vec::new();
        let mut seen = HashSet::new();

        append_investment_ids(&mut investment_ids, &mut seen, investments);

        assert_eq!(investment_ids, vec!["first", "second"]);
        assert!(has_next_page(1, PAGE_SIZE as usize, Some(101)));
        assert!(has_next_page(1, 1, Some(101)));
        assert!(!has_next_page(2, 1, Some(101)));
    }

    #[test]
    fn earn_discovery_uses_the_documented_json_post_body() {
        let asset = address!("55d398326f99059fF775485246999027B3197955");
        let request = reqwest::Client::new()
            .post("https://web3.binance.com/build/api/v1/defi/data/investment/list")
            .json(&earn_list_request(asset, 1))
            .build()
            .unwrap_or_else(|error| panic!("request setup failed: {error}"));
        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .and_then(|body| std::str::from_utf8(body).ok())
            .unwrap_or_else(|| panic!("JSON request body was not buffered"));

        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            body,
            concat!(
                r#"{"investType":"Earn","tokenAddressList":["0x55d398326f99059fF775485246999027B3197955"],"#,
                r#""binanceChainId":"56","sortField":"apy","sortDirection":"DESC","page":1,"size":100}"#,
            )
        );
    }
}
