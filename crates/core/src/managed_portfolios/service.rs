use std::sync::Arc;

use super::{
    ListManagedPortfolioTemplatesQuery, ManagedPortfolioProviderError, ManagedPortfolioReader,
    ManagedPortfolioTemplatePage,
};

pub struct ManagedPortfolioService {
    reader: Arc<dyn ManagedPortfolioReader>,
}

impl ManagedPortfolioService {
    #[must_use]
    pub fn new(reader: Arc<dyn ManagedPortfolioReader>) -> Self {
        Self { reader }
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
        self.reader.templates(query).await
    }
}
