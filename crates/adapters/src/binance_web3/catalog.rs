use std::time::Duration;

use async_trait::async_trait;
use binance_web3_rwa::apis::{
    configuration::Configuration,
    rwa_data_api::{
        GetRwaTokenListParams, GetRwaTokenPriceParams, GetRwaUnderlyingInfoParams,
        GetRwaUnderlyingMarketDataParams, get_rwa_token_list, get_rwa_token_price,
        get_rwa_underlying_info, get_rwa_underlying_market_data,
    },
};
use reqwest_middleware::ClientBuilder;
use stockfront_core::bstock_market::{
    BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockSummary, CatalogError,
};

use super::{
    BSTOCK_PLATFORM_ID,
    auth::{BinanceConfigurationError, BinanceWeb3Authentication},
    mapping::{ProviderDetails, ProviderToken, price_matches_contract},
};

#[derive(Clone)]
pub struct BinanceRwaCatalog {
    configuration: Configuration,
}

impl BinanceRwaCatalog {
    /// Creates an authenticated Binance Web3 RWA catalog.
    ///
    /// # Errors
    ///
    /// Returns [`BinanceConfigurationError`] when credentials cannot be
    /// represented safely as headers or the HTTP client cannot be built.
    pub fn new(
        base_url: &str,
        api_key: &str,
        api_secret: &str,
        timeout: Duration,
    ) -> Result<Self, BinanceConfigurationError> {
        let authentication = BinanceWeb3Authentication::new(api_key, api_secret)?;
        let client = reqwest::Client::builder().timeout(timeout).build()?;
        let client = ClientBuilder::new(client).with(authentication).build();
        let configuration = Configuration {
            base_path: base_url.trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client,
            ..Configuration::default()
        };
        Ok(Self { configuration })
    }

    async fn token_list(&self) -> Result<Vec<BstockSummary>, CatalogError> {
        let response = get_rwa_token_list(
            &self.configuration,
            GetRwaTokenListParams {
                binance_chain_id: Some(BNB_CHAIN_ID.to_string()),
                platform_id: Some(BSTOCK_PLATFORM_ID.to_owned()),
                tab_id: None,
            },
        )
        .await
        .map_err(|_| CatalogError::Unavailable)?;
        ensure_business_success(response.code, response.success)?;

        response
            .data
            .unwrap_or_default()
            .into_iter()
            .map(ProviderToken::from)
            .map(BstockSummary::try_from)
            .collect()
    }

    async fn detail_data(&self, summary: BstockSummary) -> Result<ProviderDetails, CatalogError> {
        let id = summary.id;
        let chain_id = id.chain_id.to_string();
        let contract = id.contract.to_string();
        let profile_params = GetRwaUnderlyingInfoParams {
            binance_chain_id: chain_id.clone(),
            token_contract_address: contract.clone(),
        };
        let market_params = GetRwaUnderlyingMarketDataParams {
            binance_chain_id: chain_id.clone(),
            token_contract_address: contract.clone(),
        };
        let price_params = GetRwaTokenPriceParams {
            binance_chain_id: chain_id,
            token_contract_addresses: contract,
        };
        let profile_request = async {
            get_rwa_underlying_info(&self.configuration, profile_params)
                .await
                .map_err(|_| CatalogError::Unavailable)
        };
        let market_request = async {
            get_rwa_underlying_market_data(&self.configuration, market_params)
                .await
                .map_err(|_| CatalogError::Unavailable)
        };
        let price_request = async {
            get_rwa_token_price(&self.configuration, price_params)
                .await
                .map_err(|_| CatalogError::Unavailable)
        };
        let (profile_response, market_response, price_response) =
            tokio::try_join!(profile_request, market_request, price_request)?;

        ensure_business_success(profile_response.code, profile_response.success)?;
        ensure_business_success(market_response.code, market_response.success)?;
        ensure_business_success(price_response.code, price_response.success)?;

        let profile =
            profile_response
                .data
                .map(|value| *value)
                .ok_or(CatalogError::InvalidData {
                    field: "underlying profile",
                })?;
        let market = market_response
            .data
            .map(|value| *value)
            .ok_or(CatalogError::InvalidData {
                field: "underlying market",
            })?;
        let price = price_response
            .data
            .unwrap_or_default()
            .into_iter()
            .find(|value| price_matches_contract(value, id.contract))
            .ok_or(CatalogError::InvalidData {
                field: "token price",
            })?;

        Ok(ProviderDetails::new(summary, profile, market, price))
    }
}

#[async_trait]
impl BstockCatalog for BinanceRwaCatalog {
    async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
        self.token_list().await
    }

    async fn find_bstock(&self, id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
        let Some(summary) = self
            .token_list()
            .await?
            .into_iter()
            .find(|asset| asset.id == id)
        else {
            return Ok(None);
        };
        self.detail_data(summary)
            .await
            .and_then(BstockDetails::try_from)
            .map(Some)
    }
}

fn ensure_business_success(code: Option<i32>, success: Option<bool>) -> Result<(), CatalogError> {
    if code == Some(0) && success != Some(false) {
        Ok(())
    } else {
        Err(CatalogError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_error_in_http_success_response_is_rejected() {
        assert_eq!(
            ensure_business_success(Some(40_001), Some(false)),
            Err(CatalogError::Unavailable)
        );
    }
}
