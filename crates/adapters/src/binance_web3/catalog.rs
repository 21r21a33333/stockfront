use std::sync::Arc;

use async_trait::async_trait;
use binance_web3_rwa::apis::rwa_data_api::{
    GetRwaTokenListParams, GetRwaTokenPriceParams, GetRwaUnderlyingInfoParams,
    GetRwaUnderlyingMarketDataParams, get_rwa_token_list, get_rwa_token_price,
    get_rwa_underlying_info, get_rwa_underlying_market_data,
};
use stockfront_core::bstock_market::{
    BNB_CHAIN_ID, BstockCatalog, BstockDetails, BstockId, BstockSummary, CatalogError,
};

use super::{
    BSTOCK_PLATFORM_ID,
    client::BinanceWeb3Client,
    mapping::{ProviderDetails, ProviderToken},
    provider::business_succeeded,
};

impl BinanceWeb3Client {
    /// Returns the bStock listing, reading it from the provider at most once per
    /// cache window however many services ask for it within that window.
    async fn token_list(&self) -> Result<Arc<Vec<BstockSummary>>, CatalogError> {
        self.bstocks
            .get_or_load((), async { self.read_token_list().await.map(Arc::new) })
            .await
    }

    async fn read_token_list(&self) -> Result<Vec<BstockSummary>, CatalogError> {
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
        if !business_succeeded(response.code, response.success) {
            return Err(CatalogError::Unavailable);
        }

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

        if !business_succeeded(profile_response.code, profile_response.success)
            || !business_succeeded(market_response.code, market_response.success)
            || !business_succeeded(price_response.code, price_response.success)
        {
            return Err(CatalogError::Unavailable);
        }

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
            .next()
            .ok_or(CatalogError::InvalidData {
                field: "token price",
            })?;

        Ok(ProviderDetails::new(summary, profile, market, price))
    }
}

#[async_trait]
impl BstockCatalog for BinanceWeb3Client {
    async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
        self.token_list()
            .await
            .map(|assets| assets.as_ref().clone())
    }

    async fn find_bstock(&self, id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
        let Some(summary) = self
            .token_list()
            .await?
            .iter()
            .find(|asset| asset.id == id)
            .cloned()
        else {
            return Ok(None);
        };
        self.detail_data(summary)
            .await
            .and_then(BstockDetails::try_from)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_error_in_http_success_response_is_rejected() {
        assert!(!business_succeeded(Some(40_001), Some(false)));
    }
}
