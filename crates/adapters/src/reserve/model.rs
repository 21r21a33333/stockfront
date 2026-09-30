use rust_decimal::Decimal;
use serde::Deserialize;
use stockfront_core::baskets::{
    BasketAccount, BasketAssetAllocation, BasketId, BasketKind, BasketPosition,
    BasketProviderError, BasketSummary,
};
use stockfront_core::bstock_market::BNB_CHAIN_ID;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReserveDtf {
    pub(super) address: String,
    pub(super) chain_id: u64,
    pub(super) status: Option<String>,
    #[serde(rename = "type")]
    pub(super) kind: Option<String>,
    pub(super) name: String,
    pub(super) symbol: String,
    pub(super) price: Option<Decimal>,
    pub(super) market_cap: Option<Decimal>,
    pub(super) fee: Option<Decimal>,
    #[serde(default)]
    pub(super) basket: Vec<ReserveBasketAsset>,
    pub(super) brand: Option<ReserveBrand>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReserveBasketAsset {
    pub(super) address: String,
    pub(super) symbol: String,
    pub(super) name: String,
    pub(super) weight: Decimal,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReserveBrand {
    pub(super) icon: Option<String>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    pub(super) about: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReservePortfolio {
    #[serde(rename = "indexDTFs", default)]
    pub(super) index_dtfs: Vec<ReservePortfolioPosition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReservePortfolioPosition {
    pub(super) address: String,
    pub(super) chain_id: u64,
    pub(super) name: String,
    pub(super) symbol: String,
    pub(super) amount: String,
    pub(super) value: Decimal,
}

impl ReservePortfolio {
    pub(super) fn into_account(
        self,
        observed_at_ms: i64,
    ) -> Result<BasketAccount, BasketProviderError> {
        let mut positions = self
            .index_dtfs
            .into_iter()
            .filter(|position| position.chain_id == BNB_CHAIN_ID)
            .map(TryInto::try_into)
            .collect::<Result<Vec<BasketPosition>, _>>()?;
        positions.retain(|position| !position.token_amount.is_zero());
        positions.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.symbol.cmp(&right.symbol))
        });
        let total_value_usd = positions
            .iter()
            .try_fold(Decimal::ZERO, |total, position| {
                total
                    .checked_add(position.value_usd)
                    .ok_or(BasketProviderError::InvalidData)
            })?;

        Ok(BasketAccount {
            chain_id: BNB_CHAIN_ID,
            positions,
            total_value_usd,
            observed_at_ms,
        })
    }
}

impl TryFrom<ReservePortfolioPosition> for BasketPosition {
    type Error = BasketProviderError;

    fn try_from(value: ReservePortfolioPosition) -> Result<Self, Self::Error> {
        let token_amount = value
            .amount
            .parse::<Decimal>()
            .map_err(|_| BasketProviderError::InvalidData)?;
        if token_amount.is_sign_negative() || value.value.is_sign_negative() {
            return Err(BasketProviderError::InvalidData);
        }

        Ok(Self {
            id: BasketId::bnb(
                value
                    .address
                    .parse()
                    .map_err(|_| BasketProviderError::InvalidData)?,
            ),
            kind: BasketKind::IndexToken,
            provider_id: "reserve".to_owned(),
            provider_name: "Reserve".to_owned(),
            name: value.name,
            symbol: value.symbol,
            token_amount,
            value_usd: value.value,
        })
    }
}

impl TryFrom<ReserveDtf> for BasketSummary {
    type Error = BasketProviderError;

    fn try_from(value: ReserveDtf) -> Result<Self, Self::Error> {
        let (categories, description, image_url) = value.brand.map_or_else(
            || (Vec::new(), None, None),
            |brand| {
                (
                    brand.tags,
                    non_empty(brand.about.as_deref()),
                    non_empty(brand.icon.as_deref()),
                )
            },
        );

        Ok(Self {
            id: BasketId::bnb(
                value
                    .address
                    .parse()
                    .map_err(|_| BasketProviderError::InvalidData)?,
            ),
            kind: BasketKind::IndexToken,
            provider_id: "reserve".to_owned(),
            provider_name: "Reserve".to_owned(),
            name: value.name,
            symbol: value.symbol,
            price_usd: value.price,
            market_cap_usd: value.market_cap,
            annual_fee_percent: value.fee,
            assets: value
                .basket
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            categories,
            description,
            image_url,
        })
    }
}

impl TryFrom<ReserveBasketAsset> for BasketAssetAllocation {
    type Error = BasketProviderError;

    fn try_from(value: ReserveBasketAsset) -> Result<Self, Self::Error> {
        Ok(Self {
            contract: value
                .address
                .parse()
                .map_err(|_| BasketProviderError::InvalidData)?,
            symbol: value.symbol,
            name: value.name,
            weight_percent: value.weight,
        })
    }
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_exact_decimal_values_and_omits_empty_brand_fields() {
        let basket = BasketSummary::try_from(ReserveDtf {
            address: "0xa0Fe4e0aEca5479705ce996615B2EACB6b6a10Fb".to_owned(),
            chain_id: BNB_CHAIN_ID,
            status: Some("active".to_owned()),
            kind: Some("index".to_owned()),
            name: "Reserve AI Photonics DTF".to_owned(),
            symbol: "PHOTON".to_owned(),
            price: Some(Decimal::new(85_202_230, 6)),
            market_cap: Some(Decimal::new(2_300_001, 0)),
            fee: Some(Decimal::new(3, 1)),
            basket: vec![ReserveBasketAsset {
                address: "0x0C50323AF81D5C33822c6Add256Fb9093D42BC74".to_owned(),
                symbol: "AXTIon".to_owned(),
                name: "AXT (Ondo Tokenized)".to_owned(),
                weight: Decimal::new(158, 2),
            }],
            brand: Some(ReserveBrand {
                icon: Some(String::new()),
                tags: vec!["AI".to_owned(), "RWA".to_owned()],
                about: Some("  ".to_owned()),
            }),
        })
        .unwrap_or_else(|error| panic!("valid Reserve DTF rejected: {error}"));

        assert_eq!(basket.price_usd, Some(Decimal::new(85_202_230, 6)));
        assert_eq!(basket.assets[0].weight_percent, Decimal::new(158, 2));
        assert_eq!(basket.categories, ["AI", "RWA"]);
        assert_eq!(basket.description, None);
        assert_eq!(basket.image_url, None);
    }

    #[test]
    fn maps_bnb_positions_with_exact_values_and_deterministic_order() {
        let account = ReservePortfolio {
            index_dtfs: vec![
                portfolio_position(BNB_CHAIN_ID, 2, "Robotics", "ROBOTS", "2.25", "9.75"),
                portfolio_position(8453, 3, "Base Basket", "BASE", "4", "20"),
                portfolio_position(BNB_CHAIN_ID, 4, "Zero", "ZERO", "0", "0"),
                portfolio_position(BNB_CHAIN_ID, 1, "AI", "AI", "1.5", "10.25"),
            ],
        }
        .into_account(1_234)
        .unwrap_or_else(|error| panic!("valid Reserve portfolio rejected: {error}"));

        assert_eq!(account.chain_id, BNB_CHAIN_ID);
        assert_eq!(account.observed_at_ms, 1_234);
        assert_eq!(account.positions.len(), 2);
        assert_eq!(account.positions[0].symbol, "AI");
        assert_eq!(account.positions[0].token_amount, Decimal::new(15, 1));
        assert_eq!(account.positions[1].symbol, "ROBOTS");
        assert_eq!(account.total_value_usd, Decimal::new(20, 0));
    }

    #[test]
    fn rejects_malformed_or_negative_bnb_positions() {
        for position in [
            ReservePortfolioPosition {
                address: "not-an-address".to_owned(),
                ..portfolio_position(BNB_CHAIN_ID, 1, "AI", "AI", "1", "1")
            },
            portfolio_position(BNB_CHAIN_ID, 1, "AI", "AI", "not-a-number", "1"),
            portfolio_position(BNB_CHAIN_ID, 1, "AI", "AI", "-1", "1"),
            portfolio_position(BNB_CHAIN_ID, 1, "AI", "AI", "1", "-1"),
        ] {
            assert_eq!(
                ReservePortfolio {
                    index_dtfs: vec![position]
                }
                .into_account(1),
                Err(BasketProviderError::InvalidData)
            );
        }
    }

    fn portfolio_position(
        chain_id: u64,
        address_last_byte: u8,
        name: &str,
        symbol: &str,
        amount: &str,
        value: &str,
    ) -> ReservePortfolioPosition {
        ReservePortfolioPosition {
            address: alloy_primitives::Address::with_last_byte(address_last_byte).to_string(),
            chain_id,
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            amount: amount.to_owned(),
            value: value
                .parse()
                .unwrap_or_else(|error| panic!("invalid test decimal: {error}")),
        }
    }
}
