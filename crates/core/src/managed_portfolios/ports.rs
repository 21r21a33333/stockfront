use async_trait::async_trait;
use thiserror::Error;

use super::{
    EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery, ListManagedPortfoliosQuery,
    ManagedPortfolioEnrollment, ManagedPortfolioPage, ManagedPortfolioPositions,
    ManagedPortfolioTemplatePage, PrepareManagedPortfolioEnrollmentRequest,
    PreparedManagedPortfolioEnrollment,
};
use crate::wallet_portfolio::WalletAddress;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ManagedPortfolioProviderError {
    #[error("the managed portfolio provider is temporarily unavailable")]
    Unavailable,
    #[error("the managed portfolio provider returned invalid data")]
    InvalidData,
    #[error("the managed portfolio provider rejected the request")]
    Rejected,
    #[error("the managed portfolio enrollment conflicts with existing state")]
    Conflict,
    #[error("the managed portfolio was not found")]
    NotFound,
}

#[async_trait]
pub trait ManagedPortfolioGateway: Send + Sync {
    async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ManagedPortfolioTemplatePage, ManagedPortfolioProviderError>;

    async fn prepare_enrollment(
        &self,
        request: &PrepareManagedPortfolioEnrollmentRequest,
    ) -> Result<PreparedManagedPortfolioEnrollment, ManagedPortfolioProviderError>;

    async fn enroll(
        &self,
        request: &EnrollManagedPortfolioRequest,
    ) -> Result<ManagedPortfolioEnrollment, ManagedPortfolioProviderError>;

    async fn portfolios(
        &self,
        wallet: WalletAddress,
        query: &ListManagedPortfoliosQuery,
    ) -> Result<ManagedPortfolioPage, ManagedPortfolioProviderError>;

    async fn positions(
        &self,
        portfolio_id: &str,
    ) -> Result<ManagedPortfolioPositions, ManagedPortfolioProviderError>;
}
