use async_trait::async_trait;
use stockfront_core::{
    baskets::{BasketCatalog, BasketProviderError, BasketSummary},
    bstock_market::BNB_CHAIN_ID,
};

use super::{
    client::{ReserveClient, ReserveConfigurationError},
    model::ReserveDtf,
};

pub struct ReserveAdapter {
    pub(super) client: ReserveClient,
    pub(super) slippage_denominator: rust_decimal::Decimal,
}

impl ReserveAdapter {
    /// Creates a Reserve adapter backed by the current discovery and 1inch Fusion APIs.
    ///
    /// # Errors
    ///
    /// Returns [`ReserveConfigurationError`] when the API client cannot be configured.
    pub fn new(
        base_url: &str,
        timeout: std::time::Duration,
        trade_slippage_bps: u32,
    ) -> Result<Self, ReserveConfigurationError> {
        if !(1..=10_000).contains(&trade_slippage_bps) {
            return Err(ReserveConfigurationError::InvalidSlippage);
        }
        Ok(Self {
            client: ReserveClient::new(base_url, timeout)?,
            slippage_denominator: rust_decimal::Decimal::from(10_000_u32)
                / rust_decimal::Decimal::from(trade_slippage_bps),
        })
    }
}

#[async_trait]
impl BasketCatalog for ReserveAdapter {
    async fn list_baskets(&self) -> Result<Vec<BasketSummary>, BasketProviderError> {
        self.client
            .discover()
            .await?
            .into_iter()
            .filter(is_active_bnb_index)
            .map(TryInto::try_into)
            .collect()
    }
}

fn is_active_bnb_index(dtf: &ReserveDtf) -> bool {
    dtf.chain_id == BNB_CHAIN_ID
        && dtf.status.as_deref().unwrap_or("active") == "active"
        && matches!(dtf.kind.as_deref(), None | Some("index"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_active_bnb_index_records() {
        let mut dtf = record();
        assert!(is_active_bnb_index(&dtf));

        dtf.chain_id = 8453;
        assert!(!is_active_bnb_index(&dtf));
        dtf.chain_id = BNB_CHAIN_ID;
        dtf.status = Some("deprecated".to_owned());
        assert!(!is_active_bnb_index(&dtf));
        dtf.status = Some("active".to_owned());
        dtf.kind = Some("yield".to_owned());
        assert!(!is_active_bnb_index(&dtf));
    }

    fn record() -> ReserveDtf {
        ReserveDtf {
            address: "0xa0Fe4e0aEca5479705ce996615B2EACB6b6a10Fb".to_owned(),
            chain_id: BNB_CHAIN_ID,
            status: Some("active".to_owned()),
            kind: Some("index".to_owned()),
            name: "Reserve AI Photonics DTF".to_owned(),
            symbol: "PHOTON".to_owned(),
            price: None,
            market_cap: None,
            fee: None,
            basket: Vec::new(),
            brand: None,
        }
    }
}
