use std::{collections::HashSet, sync::Arc};

use alloy_primitives::Address;
use async_trait::async_trait;
use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingToken,
    bstock_market::BNB_CHAIN_ID,
    carry::{CarryAccount, EarnOpportunity, EarnProviderError, EarnRateType, EarnReader},
    wallet_portfolio::WalletAddress,
};

use super::{
    super::{
        client::{BinanceWeb3Client, InvestmentListKind},
        provider::{InvalidProviderData, parse, required, required_non_empty},
    },
    investments::INVESTMENT_PAGE_SIZE,
};

const EARN_INVESTMENT_TYPE: &str = "Earn";

#[async_trait]
impl EarnReader for BinanceWeb3Client {
    async fn opportunities(
        &self,
        asset: Address,
    ) -> Result<Vec<EarnOpportunity>, EarnProviderError> {
        let investment_ids = self.earn_investment_ids(asset).await?;
        if investment_ids.is_empty() {
            return Ok(Vec::new());
        }
        let details = self
            .investment_details::<EarnProviderError>(&investment_ids)
            .await?;
        let mut opportunities = Vec::with_capacity(details.len());

        for (investment_id, detail) in investment_ids.iter().zip(details) {
            let Some(detail) = detail.filter(|detail| is_bsc_earn(&detail.investment)) else {
                continue;
            };
            if let Ok(opportunity) = EarnOpportunity::try_from(ProviderEarnOpportunity {
                investment_id: investment_id.clone(),
                asset,
                observed_at_ms: detail.observed_at_ms,
                detail: detail.investment.clone(),
            }) {
                opportunities.push(opportunity);
            }
        }

        if opportunities.is_empty() {
            Err(EarnProviderError::InvalidData)
        } else {
            Ok(opportunities)
        }
    }

    async fn opportunity(
        &self,
        investment_id: &str,
    ) -> Result<Option<EarnOpportunity>, EarnProviderError> {
        let Some(detail) = self
            .investment_detail::<EarnProviderError>(investment_id)
            .await?
        else {
            return Ok(None);
        };
        if !is_bsc_earn(&detail.investment) {
            return Ok(None);
        }
        let asset = single_asset(&detail.investment)?;
        EarnOpportunity::try_from(ProviderEarnOpportunity {
            investment_id: investment_id.to_owned(),
            asset,
            observed_at_ms: detail.observed_at_ms,
            detail: detail.investment.clone(),
        })
        .map(Some)
        .map_err(Into::into)
    }

    async fn positions(&self, wallet: WalletAddress) -> Result<CarryAccount, EarnProviderError> {
        self.earn_positions(wallet).await
    }
}

impl BinanceWeb3Client {
    async fn earn_investment_ids(
        &self,
        asset: Address,
    ) -> Result<Arc<Vec<String>>, EarnProviderError> {
        self.investment_ids::<EarnProviderError, _>(InvestmentListKind::Earn, asset, |page| {
            earn_list_request(asset, page)
        })
        .await
    }

    pub(super) async fn is_bsc_earn_investment(
        &self,
        investment_id: &str,
    ) -> Result<bool, EarnProviderError> {
        Ok(self
            .investment_detail::<EarnProviderError>(investment_id)
            .await?
            .is_some_and(|detail| is_bsc_earn(&detail.investment)))
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
    request.size = Some(INVESTMENT_PAGE_SIZE);
    request
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

fn is_bsc_earn(investment: &models::DefiInvestmentDetail) -> bool {
    investment.binance_chain_id.as_deref() == Some("56")
        && investment.invest_type.as_deref() == Some(EARN_INVESTMENT_TYPE)
}

fn single_asset(detail: &models::DefiInvestmentDetail) -> Result<Address, InvalidProviderData> {
    let assets = detail
        .asset_token_list
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|token| token.token_address.as_deref())
        .map(parse)
        .collect::<Result<HashSet<Address>, _>>()?;
    if assets.len() != 1 {
        return Err(InvalidProviderData);
    }
    assets.into_iter().next().ok_or(InvalidProviderData)
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn maps_apy_and_optional_fields_without_float_conversion() {
        let asset = address!("55d398326f99059fF775485246999027B3197955");
        let investment = investment(asset, "APY", "7.25%", 725);

        let opportunity = EarnOpportunity::try_from(ProviderEarnOpportunity {
            investment_id: "investment".to_owned(),
            asset,
            observed_at_ms: 123,
            detail: investment,
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
        let mut investment = investment(asset, "APR", "5.00%", 500);
        investment.tvl = Some("unknown".to_owned());

        let opportunity = EarnOpportunity::try_from(ProviderEarnOpportunity {
            investment_id: "investment".to_owned(),
            asset,
            observed_at_ms: 123,
            detail: investment,
        })
        .unwrap_or_else(|_| panic!("valid fixture must map"));

        assert_eq!(opportunity.rate_type, EarnRateType::Apr);
        assert_eq!(opportunity.reported_tvl_usd, None);
    }

    #[test]
    fn direct_lookup_accepts_only_bsc_earn_products() {
        let mut detail = models::DefiInvestmentDetail::new();
        detail.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        detail.invest_type = Some(EARN_INVESTMENT_TYPE.to_owned());
        assert!(is_bsc_earn(&detail));

        detail.binance_chain_id = Some("1".to_owned());
        assert!(!is_bsc_earn(&detail));
        detail.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        detail.invest_type = Some("LiquidityPool".to_owned());
        assert!(!is_bsc_earn(&detail));
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

    fn investment(
        asset: Address,
        rate_type: &str,
        rate_display: &str,
        rate_bps: i64,
    ) -> models::DefiInvestmentDetail {
        let mut investment = models::DefiInvestmentDetail::new();
        investment.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        investment.defi_protocol_id = Some("venus".to_owned());
        investment.protocol_name = Some("Venus".to_owned());
        investment.investment_name = Some("USDT Supply".to_owned());
        investment.invest_type = Some(EARN_INVESTMENT_TYPE.to_owned());
        investment.investable = Some(true);
        investment.apy_bps = Some(rate_bps);
        investment.apy_display = Some(rate_display.to_owned());
        investment.apy_type = Some(rate_type.to_owned());
        investment.tvl = Some("1234.56".to_owned());
        let mut token = models::DefiInvestmentToken::new();
        token.token_address = Some(asset.to_string());
        token.token_symbol = Some("USDT".to_owned());
        investment.asset_token_list = Some(vec![token]);
        investment
    }
}
