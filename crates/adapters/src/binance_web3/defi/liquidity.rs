use alloy_primitives::Address;
use async_trait::async_trait;
use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingToken,
    bstock_market::BNB_CHAIN_ID,
    carry::EarnRateType,
    liquidity::{LiquidityAccount, LiquidityOpportunity, LiquidityProviderError, LiquidityReader},
    wallet_portfolio::WalletAddress,
};

use super::{
    super::{
        client::BinanceWeb3Client,
        provider::{InvalidProviderData, parse, parse_optional, required, required_non_empty},
    },
    investments::INVESTMENT_PAGE_SIZE,
};

#[async_trait]
impl LiquidityReader for BinanceWeb3Client {
    async fn opportunities(
        &self,
        bstock_token: Address,
    ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError> {
        let investment_ids = self
            .investment_ids::<LiquidityProviderError, _>(|page| {
                liquidity_list_request(bstock_token, page)
            })
            .await?;
        if investment_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut opportunities = Vec::with_capacity(investment_ids.len());
        for investment_id in investment_ids {
            let Some(detail) = self
                .investment_detail::<LiquidityProviderError>(&investment_id)
                .await?
            else {
                continue;
            };
            if let Ok(opportunity) = LiquidityOpportunity::try_from(ProviderLiquidityOpportunity {
                investment_id,
                observed_at_ms: detail.observed_at_ms,
                detail: detail.investment,
            }) {
                opportunities.push(opportunity);
            }
        }

        if opportunities.is_empty() {
            Err(LiquidityProviderError::InvalidData)
        } else {
            Ok(opportunities)
        }
    }

    async fn positions(
        &self,
        wallet: WalletAddress,
    ) -> Result<LiquidityAccount, LiquidityProviderError> {
        self.liquidity_positions(wallet).await
    }
}

fn liquidity_list_request(bstock_token: Address, page: i32) -> models::ListDefiInvestmentsRequest {
    let mut request = models::ListDefiInvestmentsRequest::new(
        models::list_defi_investments_request::InvestType::LiquidityPool,
    );
    request.token_address_list = Some(vec![bstock_token.to_string()]);
    request.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
    request.sort_field = Some(models::list_defi_investments_request::SortField::Apy);
    request.sort_direction = Some(models::list_defi_investments_request::SortDirection::Desc);
    request.page = Some(page);
    request.size = Some(INVESTMENT_PAGE_SIZE);
    request
}

struct ProviderLiquidityOpportunity {
    investment_id: String,
    observed_at_ms: i64,
    detail: models::DefiInvestmentDetail,
}

impl TryFrom<ProviderLiquidityOpportunity> for LiquidityOpportunity {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderLiquidityOpportunity) -> Result<Self, Self::Error> {
        let assets = provider
            .detail
            .asset_token_list
            .unwrap_or_default()
            .into_iter()
            .map(ProviderLiquidityToken)
            .map(BorrowingToken::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if assets.len() != 2 {
            return Err(InvalidProviderData);
        }
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
            pool_address: parse_optional(provider.detail.pool_address.as_deref())?,
            assets,
            rate_type,
            reported_rate_bps: required(provider.detail.apy_bps)?,
            reported_rate_display: required_non_empty(provider.detail.apy_display)?,
            reported_tvl_usd: provider
                .detail
                .tvl
                .as_deref()
                .and_then(|value| parse::<Decimal>(value).ok()),
            fee_rate: provider
                .detail
                .fee_rate
                .as_deref()
                .and_then(|value| parse::<Decimal>(value).ok()),
            investable: required(provider.detail.investable)?,
            observed_at_ms: provider.observed_at_ms,
        })
    }
}

struct ProviderLiquidityToken(models::DefiInvestmentToken);

impl TryFrom<ProviderLiquidityToken> for BorrowingToken {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderLiquidityToken) -> Result<Self, Self::Error> {
        let token = provider.0;
        Ok(Self {
            contract: parse(&required_non_empty(token.token_address)?)?,
            symbol: required_non_empty(token.token_symbol)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn maps_liquidity_opportunity_without_losing_financial_precision() {
        let bstock = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
        let settlement = address!("55d398326f99059fF775485246999027B3197955");
        let opportunity = LiquidityOpportunity::try_from(ProviderLiquidityOpportunity {
            investment_id: "investment".to_owned(),
            observed_at_ms: 123,
            detail: investment(bstock, settlement),
        })
        .unwrap_or_else(|_| panic!("valid fixture must map"));

        assert_eq!(opportunity.assets[0].contract, bstock);
        assert_eq!(opportunity.assets[1].contract, settlement);
        assert_eq!(opportunity.reported_rate_bps, 725);
        assert_eq!(opportunity.reported_tvl_usd, Some(Decimal::new(123_456, 2)));
        assert_eq!(opportunity.fee_rate, Some(Decimal::new(25, 4)));
    }

    #[test]
    fn discovery_uses_binance_liquidity_pool_filter_and_bnb_chain() {
        let bstock = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
        let request = reqwest::Client::new()
            .post("https://web3.binance.com/build/api/v1/defi/data/investment/list")
            .json(&liquidity_list_request(bstock, 1))
            .build()
            .unwrap_or_else(|error| panic!("request setup failed: {error}"));
        let body = request
            .body()
            .and_then(reqwest::Body::as_bytes)
            .and_then(|body| std::str::from_utf8(body).ok())
            .unwrap_or_else(|| panic!("JSON request body was not buffered"));

        assert_eq!(
            body,
            concat!(
                r#"{"investType":"LiquidityPool","tokenAddressList":["0x5b1910eAaD6450E50f816082Aa078C41F10C292f"],"#,
                r#""binanceChainId":"56","sortField":"apy","sortDirection":"DESC","page":1,"size":100}"#,
            )
        );
    }

    fn investment(bstock: Address, settlement: Address) -> models::DefiInvestmentDetail {
        let mut investment = models::DefiInvestmentDetail::new();
        investment.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        investment.defi_protocol_id = Some("pancakeswap4".to_owned());
        investment.protocol_name = Some("PancakeSwap".to_owned());
        investment.investment_name = Some("TSLAB-USDT".to_owned());
        investment.invest_type = Some("LiquidityPool".to_owned());
        investment.investable = Some(true);
        investment.apy_bps = Some(725);
        investment.apy_display = Some("7.25%".to_owned());
        investment.apy_type = Some("APY".to_owned());
        investment.tvl = Some("1234.56".to_owned());
        investment.fee_rate = Some("0.0025".to_owned());
        investment.asset_token_list = Some(vec![token(bstock, "TSLAB"), token(settlement, "USDT")]);
        investment
    }

    fn token(contract: Address, symbol: &str) -> models::DefiInvestmentToken {
        let mut token = models::DefiInvestmentToken::new();
        token.token_address = Some(contract.to_string());
        token.token_symbol = Some(symbol.to_owned());
        token
    }
}
