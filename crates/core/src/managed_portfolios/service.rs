use std::sync::Arc;

use thiserror::Error;

use super::{
    EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery, ListManagedPortfoliosQuery,
    ManagedPortfolioEnrollment, ManagedPortfolioGateway, ManagedPortfolioOperation,
    ManagedPortfolioPage, ManagedPortfolioPositions, ManagedPortfolioProviderError,
    ManagedPortfolioTemplatePage, PrepareManagedPortfolioDepositRequest,
    PrepareManagedPortfolioEnrollmentRequest, PrepareManagedPortfolioWithdrawalRequest,
    PreparedManagedPortfolioDeposit, PreparedManagedPortfolioEnrollment,
    PreparedManagedPortfolioWithdrawal, SubmitManagedPortfolioWithdrawalRequest,
    SubmittedManagedPortfolioWithdrawal,
};
use crate::wallet_portfolio::WalletAddress;

#[derive(Debug, Error)]
pub enum ManagedPortfolioServiceError {
    #[error(transparent)]
    Provider(#[from] ManagedPortfolioProviderError),
    #[error("managed portfolio deposit amount must be greater than zero")]
    InvalidDepositAmount,
    #[error("in-kind withdrawal assets and amounts must be greater than zero")]
    InvalidWithdrawalAssets,
}

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

    /// Returns one page of managed portfolios owned by a wallet.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the provider cannot serve the portfolio list.
    pub async fn portfolios(
        &self,
        wallet: WalletAddress,
        query: &ListManagedPortfoliosQuery,
    ) -> Result<ManagedPortfolioPage, ManagedPortfolioProviderError> {
        self.gateway.portfolios(wallet, query).await
    }

    /// Returns current BNB Chain positions for a managed portfolio.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the portfolio does not exist or the provider
    /// cannot read its positions.
    pub async fn positions(
        &self,
        portfolio_id: &str,
    ) -> Result<ManagedPortfolioPositions, ManagedPortfolioProviderError> {
        self.gateway.positions(portfolio_id).await
    }

    /// Prepares an unsigned ERC-20 transfer into a managed portfolio's BNB smart account.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioServiceError`] when the amount is zero, the portfolio does not
    /// exist, or its provider cannot prepare the deposit.
    pub async fn prepare_deposit(
        &self,
        portfolio_id: &str,
        request: &PrepareManagedPortfolioDepositRequest,
    ) -> Result<PreparedManagedPortfolioDeposit, ManagedPortfolioServiceError> {
        if request.raw_amount.is_zero() {
            return Err(ManagedPortfolioServiceError::InvalidDepositAmount);
        }
        Ok(self.gateway.prepare_deposit(portfolio_id, request).await?)
    }

    /// Prepares an owner-signed authorization for an in-kind or full USDT withdrawal.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioServiceError`] when an in-kind request has no assets, contains a
    /// zero amount, or the provider cannot prepare the authorization.
    pub async fn prepare_withdrawal(
        &self,
        portfolio_id: &str,
        request: &PrepareManagedPortfolioWithdrawalRequest,
    ) -> Result<PreparedManagedPortfolioWithdrawal, ManagedPortfolioServiceError> {
        if let PrepareManagedPortfolioWithdrawalRequest::InKind { assets, .. } = request
            && (assets.is_empty() || assets.iter().any(|asset| asset.raw_amount.is_zero()))
        {
            return Err(ManagedPortfolioServiceError::InvalidWithdrawalAssets);
        }

        Ok(self
            .gateway
            .prepare_withdrawal(portfolio_id, request)
            .await?)
    }

    /// Submits a wallet-signed withdrawal authorization.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the signature or authorization is rejected,
    /// the portfolio does not exist, or the provider is unavailable.
    pub async fn submit_withdrawal(
        &self,
        portfolio_id: &str,
        request: &SubmitManagedPortfolioWithdrawalRequest,
    ) -> Result<SubmittedManagedPortfolioWithdrawal, ManagedPortfolioProviderError> {
        self.gateway.submit_withdrawal(portfolio_id, request).await
    }

    /// Returns the provider execution state for a submitted withdrawal.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedPortfolioProviderError`] when the operation does not exist or the provider
    /// cannot read it.
    pub async fn operation(
        &self,
        portfolio_id: &str,
        operation_id: &str,
    ) -> Result<ManagedPortfolioOperation, ManagedPortfolioProviderError> {
        self.gateway.operation(portfolio_id, operation_id).await
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{U256, address};
    use async_trait::async_trait;

    use super::*;

    struct UnusedGateway;

    #[async_trait]
    impl ManagedPortfolioGateway for UnusedGateway {
        async fn templates(
            &self,
            _query: &ListManagedPortfolioTemplatesQuery,
        ) -> Result<ManagedPortfolioTemplatePage, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn prepare_enrollment(
            &self,
            _request: &PrepareManagedPortfolioEnrollmentRequest,
        ) -> Result<PreparedManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn enroll(
            &self,
            _request: &EnrollManagedPortfolioRequest,
        ) -> Result<ManagedPortfolioEnrollment, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn portfolios(
            &self,
            _wallet: WalletAddress,
            _query: &ListManagedPortfoliosQuery,
        ) -> Result<ManagedPortfolioPage, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn positions(
            &self,
            _portfolio_id: &str,
        ) -> Result<ManagedPortfolioPositions, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn prepare_deposit(
            &self,
            _portfolio_id: &str,
            _request: &PrepareManagedPortfolioDepositRequest,
        ) -> Result<PreparedManagedPortfolioDeposit, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn prepare_withdrawal(
            &self,
            _portfolio_id: &str,
            _request: &PrepareManagedPortfolioWithdrawalRequest,
        ) -> Result<PreparedManagedPortfolioWithdrawal, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn submit_withdrawal(
            &self,
            _portfolio_id: &str,
            _request: &SubmitManagedPortfolioWithdrawalRequest,
        ) -> Result<SubmittedManagedPortfolioWithdrawal, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }

        async fn operation(
            &self,
            _portfolio_id: &str,
            _operation_id: &str,
        ) -> Result<ManagedPortfolioOperation, ManagedPortfolioProviderError> {
            Err(ManagedPortfolioProviderError::Unavailable)
        }
    }

    #[tokio::test]
    async fn rejects_zero_deposit_before_calling_provider() {
        let service = ManagedPortfolioService::new(Arc::new(UnusedGateway));
        let result = service
            .prepare_deposit(
                "portfolio-1",
                &PrepareManagedPortfolioDepositRequest {
                    token: address!("1111111111111111111111111111111111111111"),
                    raw_amount: U256::ZERO,
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(ManagedPortfolioServiceError::InvalidDepositAmount)
        ));
    }

    #[tokio::test]
    async fn rejects_empty_in_kind_withdrawal_before_calling_provider() {
        let service = ManagedPortfolioService::new(Arc::new(UnusedGateway));
        let result = service
            .prepare_withdrawal(
                "portfolio-1",
                &PrepareManagedPortfolioWithdrawalRequest::InKind {
                    recipient: WalletAddress::try_from(address!(
                        "1111111111111111111111111111111111111111"
                    ))
                    .unwrap_or_else(|error| panic!("test wallet must be valid: {error}")),
                    assets: Vec::new(),
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(ManagedPortfolioServiceError::InvalidWithdrawalAssets)
        ));
    }
}
