use std::time::Duration;

use glider_api::{
    apis::{
        Error as GliderApiError,
        configuration::{ApiKey, Configuration},
        discovery_api::{
            DiscoveryStrategiesListError, DiscoveryStrategiesListParams, discovery_strategies_list,
        },
    },
    models::ListDiscoveryStrategiesResponse,
};
use reqwest_middleware::ClientBuilder;
use stockfront_core::managed_portfolios::{
    ListManagedPortfolioTemplatesQuery, ManagedPortfolioCollection, ManagedPortfolioProviderError,
    ManagedPortfolioSort,
};
use thiserror::Error;
use url::Url;

pub(super) struct GliderClient {
    configuration: Configuration,
}

impl GliderClient {
    pub(super) fn new(
        base_url: &str,
        api_key: String,
        timeout: Duration,
    ) -> Result<Self, GliderConfigurationError> {
        let base_url = Url::parse(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|_| GliderConfigurationError::Unavailable)?;
        let configuration = Configuration {
            base_path: base_url.as_str().trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client: ClientBuilder::new(client).build(),
            api_key: Some(ApiKey {
                prefix: None,
                key: api_key,
            }),
            ..Configuration::default()
        };
        Ok(Self { configuration })
    }

    pub(super) async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ListDiscoveryStrategiesResponse, ManagedPortfolioProviderError> {
        let limit =
            i32::try_from(query.limit).map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        discovery_strategies_list(
            &self.configuration,
            DiscoveryStrategiesListParams {
                collection: match query.collection {
                    ManagedPortfolioCollection::Curated => "curated",
                    ManagedPortfolioCollection::TopPerforming => "top_performing",
                }
                .to_owned(),
                sort: query
                    .sort
                    .map(|sort| match sort {
                        ManagedPortfolioSort::Tvl => "tvl.desc",
                        ManagedPortfolioSort::Inflows7d => "inflows7d.desc",
                        ManagedPortfolioSort::Users => "users.desc",
                    })
                    .map(str::to_owned),
                limit: Some(limit),
                cursor: query.cursor.clone(),
            },
        )
        .await
        .map_err(|error| map_provider_error(&error))
    }
}

fn map_provider_error(
    error: &GliderApiError<DiscoveryStrategiesListError>,
) -> ManagedPortfolioProviderError {
    match error {
        GliderApiError::Serde(_) | GliderApiError::SerdePathToError(_) => {
            ManagedPortfolioProviderError::InvalidData
        }
        GliderApiError::Reqwest(_)
        | GliderApiError::ReqwestMiddleware(_)
        | GliderApiError::Io(_)
        | GliderApiError::ResponseError(_) => ManagedPortfolioProviderError::Unavailable,
    }
}

#[derive(Debug, Error)]
pub enum GliderConfigurationError {
    #[error("invalid Glider API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the Glider API client is unavailable")]
    Unavailable,
}
