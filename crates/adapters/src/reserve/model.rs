use rust_decimal::Decimal;
use serde::Deserialize;
use stockfront_core::baskets::{
    BasketAssetAllocation, BasketId, BasketKind, BasketProviderError, BasketSummary,
};

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
    use stockfront_core::bstock_market::BNB_CHAIN_ID;

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
}
