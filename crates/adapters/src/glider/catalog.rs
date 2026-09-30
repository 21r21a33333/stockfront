use std::time::Duration;

use async_trait::async_trait;
use stockfront_core::managed_portfolios::{
    EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery, ManagedPortfolioEnrollment,
    ManagedPortfolioGateway, ManagedPortfolioProviderError, ManagedPortfolioTemplate,
    ManagedPortfolioTemplatePage, PrepareManagedPortfolioEnrollmentRequest,
    PreparedManagedPortfolioEnrollment,
};

use super::{
    client::{GliderClient, GliderConfigurationError},
    model::{
        BnbManagedPortfolioEnrollment, BnbManagedPortfolioTemplate,
        BnbPreparedManagedPortfolioEnrollment,
    },
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
impl ManagedPortfolioGateway for GliderAdapter {
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

    async fn prepare_enrollment(
        &self,
        request: &PrepareManagedPortfolioEnrollmentRequest,
    ) -> Result<PreparedManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
        BnbPreparedManagedPortfolioEnrollment::try_from(
            *self.client.prepare_enrollment(request).await?.data,
        )
        .map(PreparedManagedPortfolioEnrollment::from)
    }

    async fn enroll(
        &self,
        request: &EnrollManagedPortfolioRequest,
    ) -> Result<ManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
        BnbManagedPortfolioEnrollment::try_from(*self.client.enroll(request).await?.data)
            .map(ManagedPortfolioEnrollment::from)
    }
}
