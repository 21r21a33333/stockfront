use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::bstock_market::{
    Address, AssetKind, BNB_CHAIN_ID, BstockDetails, BstockId, BstockSummary, CatalogError,
    CompanyProfile, MarketMetrics, MarketSession, MarketStatus, Protection,
};

pub(super) struct ProviderToken(models::RwaToken);

impl From<models::RwaToken> for ProviderToken {
    fn from(value: models::RwaToken) -> Self {
        Self(value)
    }
}

impl TryFrom<ProviderToken> for BstockSummary {
    type Error = CatalogError;

    fn try_from(value: ProviderToken) -> Result<Self, Self::Error> {
        let token = value.0;
        let contract = required(token.token_contract_address, "token contract")?
            .parse::<Address>()
            .map_err(|_| invalid_data("token contract"))?;
        let token_decimals = required(token.decimals, "token decimals")?
            .parse::<u8>()
            .map_err(|_| invalid_data("token decimals"))?;
        let token_to_share_ratio = required(token.token_to_share_ratio, "token-to-share ratio")?;

        Ok(Self {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract,
            },
            token_name: required(token.token_name, "token name")?,
            token_symbol: required(token.token_symbol, "token symbol")?,
            token_logo_url: token.token_logo_url,
            token_decimals,
            underlying_ticker: required(token.underlying_ticker, "underlying ticker")?,
            underlying_name: required(token.underlying_name, "underlying name")?,
            asset_kind: AssetKind::try_from(ProviderAssetKind(token.asset_type))?,
            token_to_share_ratio: parse_decimal(&token_to_share_ratio, "token-to-share ratio")?,
            token_price_usd: parse_optional_decimal(token.token_price.as_deref(), "token price")?,
            reference_price_usd: parse_optional_decimal(
                token.reference_price.as_deref(),
                "reference price",
            )?,
            volume_24h_usd: parse_optional_decimal(token.volume24_h.as_deref(), "24-hour volume")?,
            market_cap_usd: parse_optional_decimal(token.market_cap.as_deref(), "market cap")?,
            market_status: token
                .status_info
                .as_deref()
                .map(ProviderMarketStatus)
                .map(MarketStatus::from),
            tags: token.tags.unwrap_or_default(),
        })
    }
}

pub(super) struct ProviderDetails {
    summary: BstockSummary,
    profile: models::RwaUnderlyingInfo,
    market: models::RwaUnderlyingMarketData,
    price: models::RwaTokenPrice,
}

impl ProviderDetails {
    pub(super) fn new(
        summary: BstockSummary,
        profile: models::RwaUnderlyingInfo,
        market: models::RwaUnderlyingMarketData,
        price: models::RwaTokenPrice,
    ) -> Self {
        Self {
            summary,
            profile,
            market,
            price,
        }
    }
}

impl TryFrom<ProviderDetails> for BstockDetails {
    type Error = CatalogError;

    fn try_from(value: ProviderDetails) -> Result<Self, Self::Error> {
        let ProviderDetails {
            mut summary,
            profile,
            market,
            price,
        } = value;

        summary.token_price_usd =
            parse_optional_decimal(price.token_price.as_deref(), "token price")?;
        summary.reference_price_usd =
            parse_optional_decimal(price.reference_price.as_deref(), "reference price")?;
        if let Some(status) = market.status_info.as_deref() {
            summary.market_status = Some(MarketStatus::from(ProviderMarketStatus(status)));
        }

        let company = profile
            .company_info
            .map(|company| ProviderCompanyProfile(*company))
            .map(CompanyProfile::from);
        let mut protections = profile
            .protections
            .unwrap_or_default()
            .into_iter()
            .map(|(kind, protection)| Protection::from(ProviderProtection { kind, protection }))
            .collect::<Vec<_>>();
        protections.sort_by(|left, right| left.kind.cmp(&right.kind));

        Ok(Self {
            summary,
            company,
            protections,
            market_metrics: MarketMetrics::try_from(ProviderMarketMetrics(
                market.market_data.map(|value| *value),
            ))?,
            price_updated_at_ms: price.token_price_updated_at,
        })
    }
}

struct ProviderAssetKind(Option<i32>);

impl TryFrom<ProviderAssetKind> for AssetKind {
    type Error = CatalogError;

    fn try_from(value: ProviderAssetKind) -> Result<Self, Self::Error> {
        match value.0 {
            Some(1) => Ok(Self::Stock),
            Some(2) => Ok(Self::PreIpo),
            Some(3) => Ok(Self::Etf),
            _ => Err(invalid_data("asset type")),
        }
    }
}

struct ProviderMarketStatus<'a>(&'a models::MarketStatusInfo);

impl From<ProviderMarketStatus<'_>> for MarketStatus {
    fn from(value: ProviderMarketStatus<'_>) -> Self {
        let value = value.0;
        let session = match value.market_status.as_deref() {
            Some("premarket") => MarketSession::PreMarket,
            Some("regular") => MarketSession::Regular,
            Some("postmarket") => MarketSession::PostMarket,
            Some("overnight") => MarketSession::Overnight,
            Some("closed") => MarketSession::Closed,
            Some("pause") => MarketSession::Paused,
            Some(other) => MarketSession::Unknown(other.to_owned()),
            None => MarketSession::Unknown("unknown".to_owned()),
        };

        Self {
            session,
            tradable: value.open_state.unwrap_or(false),
            reason_code: value.reason_code.clone().flatten(),
            reason: value.reason_msg.clone().flatten(),
            next_open_at_ms: value.next_open_time.flatten(),
            next_close_at_ms: value.next_close_time.flatten(),
        }
    }
}

struct ProviderCompanyProfile(models::CompanyInfo);

impl From<ProviderCompanyProfile> for CompanyProfile {
    fn from(value: ProviderCompanyProfile) -> Self {
        let value = value.0;
        Self {
            ceo: value.ceo,
            website: value.website,
            industry: value.industry,
            concepts: value.concepts_en.unwrap_or_default(),
            description: value.description_en,
        }
    }
}

struct ProviderProtection {
    kind: String,
    protection: models::Protection,
}

impl From<ProviderProtection> for Protection {
    fn from(value: ProviderProtection) -> Self {
        Self {
            kind: value.kind,
            supported: value.protection.supported.unwrap_or(false),
            report_url: value.protection.url,
        }
    }
}

struct ProviderMarketMetrics(Option<models::UnderlyingMarketData>);

impl TryFrom<ProviderMarketMetrics> for MarketMetrics {
    type Error = CatalogError;

    fn try_from(value: ProviderMarketMetrics) -> Result<Self, Self::Error> {
        let value = value.0.unwrap_or_default();
        Ok(Self {
            high_52w_usd: parse_optional_decimal(value.high52_w.as_deref(), "52-week high")?,
            low_52w_usd: parse_optional_decimal(value.low52_w.as_deref(), "52-week low")?,
            volume_24h_shares: parse_optional_decimal(
                value.volume_shares24_h.as_deref(),
                "share volume",
            )?,
            average_daily_volume_1y: parse_optional_decimal(
                value.avg_daily_volume1_y.as_deref(),
                "average daily volume",
            )?,
            total_shares: parse_optional_decimal(value.total_shares.as_deref(), "total shares")?,
            turnover_rate_percent: parse_optional_decimal(
                value.turnover_rate.as_deref(),
                "turnover rate",
            )?,
            pe_ratio_ttm: parse_optional_decimal(
                value.pe_ratio_ttm.as_ref().and_then(Option::as_deref),
                "PE ratio",
            )?,
            pb_ratio: parse_optional_decimal(
                value.pb_ratio.as_ref().and_then(Option::as_deref),
                "PB ratio",
            )?,
            dividend_yield_percent: parse_optional_decimal(
                value.dividend_yield.as_ref().and_then(Option::as_deref),
                "dividend yield",
            )?,
            latest_dividend_usd: parse_optional_decimal(
                value.latest_dividend.as_ref().and_then(Option::as_deref),
                "latest dividend",
            )?,
        })
    }
}

fn required(value: Option<String>, field: &'static str) -> Result<String, CatalogError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_data(field))
}

fn parse_decimal(value: &str, field: &'static str) -> Result<Decimal, CatalogError> {
    value.parse::<Decimal>().map_err(|_| invalid_data(field))
}

fn parse_optional_decimal(
    value: Option<&str>,
    field: &'static str,
) -> Result<Option<Decimal>, CatalogError> {
    value.map(|value| parse_decimal(value, field)).transpose()
}

fn invalid_data(field: &'static str) -> CatalogError {
    CatalogError::InvalidData { field }
}

pub(super) fn price_matches_contract(value: &models::RwaTokenPrice, expected: Address) -> bool {
    value
        .token_contract_address
        .as_deref()
        .and_then(|contract| contract.parse::<Address>().ok())
        == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_mapping_rejects_an_invalid_contract() {
        let mut accepted = valid_token();
        let mapped = BstockSummary::try_from(ProviderToken::from(accepted.clone()))
            .unwrap_or_else(|error| panic!("valid provider data rejected: {error}"));

        assert_eq!(mapped.underlying_ticker, "AAPL");
        accepted.token_contract_address = Some("not-an-address".to_owned());
        assert!(matches!(
            BstockSummary::try_from(ProviderToken::from(accepted)),
            Err(CatalogError::InvalidData {
                field: "token contract"
            })
        ));
    }

    fn valid_token() -> models::RwaToken {
        models::RwaToken {
            binance_chain_id: None,
            token_contract_address: Some("0x1234567890123456789012345678901234567890".to_owned()),
            platform_id: None,
            asset_type: Some(1),
            token_name: Some("Apple token".to_owned()),
            token_symbol: Some("AAPLx".to_owned()),
            token_logo_url: None,
            decimals: Some("18".to_owned()),
            underlying_ticker: Some("AAPL".to_owned()),
            underlying_name: Some("Apple".to_owned()),
            underlying_name_zh: None,
            token_to_share_ratio: Some("1.003701".to_owned()),
            tags: None,
            status_info: None,
            token_price: Some("250.12".to_owned()),
            reference_price: Some("249.20".to_owned()),
            volume24_h: Some("1000000.10".to_owned()),
            market_cap: Some("1000000000".to_owned()),
            pe_ratio_ttm: None,
        }
    }
}
