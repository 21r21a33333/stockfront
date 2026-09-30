use std::sync::Arc;

use super::{
    EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery, ManagedPortfolioEnrollment,
    ManagedPortfolioGateway, ManagedPortfolioProviderError, ManagedPortfolioTemplatePage,
    PrepareManagedPortfolioEnrollmentRequest, PreparedManagedPortfolioEnrollment,
};

pub struct ManagedPortfolioService {
    gateway: Arc<dyn ManagedPortfolioGateway>,
}

impl ManagedPortfolioService {
    #[must_use]
    pub fn new(gateway: Arc<dyn ManagedPortfolioGateway>) -> Self {
        Self { gateway }
    }

    /// Returns one page of managed portfolio templates.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the provider cannot serve the catalog.
    pub async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ManagedPortfolioTemplatePage, ManagedPortfolioProviderError> {
        self.gateway.templates(query).await
    }

    /// Prepares the wallet authorization required to enroll in a managed portfolio template.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the provider rejects or cannot prepare the
    /// enrollment.
    pub async fn prepare_enrollment(
        &self,
        request: &PrepareManagedPortfolioEnrollmentRequest,
    ) -> Result<PreparedManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
        self.gateway.prepare_enrollment(request).await
    }

    /// Creates the managed portfolio after the wallet signs its enrollment authorization.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the authorization is rejected, conflicts
    /// with existing state, or the provider is unavailable.
    pub async fn enroll(
        &self,
        request: &EnrollManagedPortfolioRequest,
    ) -> Result<ManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
        self.gateway.enroll(request).await
    }
}
