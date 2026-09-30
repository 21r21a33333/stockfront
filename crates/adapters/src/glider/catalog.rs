use std::time::Duration;

use async_trait::async_trait;
use stockfront_core::managed_portfolios::{
    ListManagedPortfolioTemplatesQuery, ManagedPortfolioProviderError, ManagedPortfolioReader,
    ManagedPortfolioTemplate, ManagedPortfolioTemplatePage,
};

use super::{
    client::{GliderClient, GliderConfigurationError},
    model::BnbManagedPortfolioTemplate,
};

pub struct GliderAdapter {
    client: GliderClient,
}

impl GliderAdapter {
    /// Creates a Glider adapter for managed portfolio discovery.
    ///
    /// # Errors
    ///
    /// Returns [`GliderConfigurationError`] when the API client cannot be configured.
    pub fn new(
        base_url: &str,
        api_key: String,
        timeout: Duration,
    ) -> Result<Self, GliderConfigurationError> {
        Ok(Self {
            client: GliderClient::new(base_url, api_key, timeout)?,
        })
    }
}

#[async_trait]
impl ManagedPortfolioReader for GliderAdapter {
    async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ManagedPortfolioTemplatePage, ManagedPortfolioProviderError> {
        let response = self.client.templates(query).await?;
        let items = response
            .data
            .strategies
            .into_iter()
            .filter_map(|strategy| BnbManagedPortfolioTemplate::try_from(strategy).ok())
            .map(ManagedPortfolioTemplate::from)
            .collect();

        Ok(ManagedPortfolioTemplatePage {
            items,
            next_cursor: response.next_cursor,
        })
    }
}
