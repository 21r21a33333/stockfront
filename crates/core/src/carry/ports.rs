use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;

use crate::{
    carry::{CarryAccount, CarryDepositPlan, EarnDepositCommand, EarnOpportunity},
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum EarnProviderError {
    #[error("earn opportunities are temporarily unavailable")]
    Unavailable,
    #[error("the earn provider rejected the request")]
    Rejected,
    #[error("the earn opportunity provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait EarnReader: Send + Sync {
    async fn opportunities(
        &self,
        asset: Address,
    ) -> Result<Vec<EarnOpportunity>, EarnProviderError>;

    async fn opportunity(
        &self,
        investment_id: &str,
    ) -> Result<Option<EarnOpportunity>, EarnProviderError>;

    async fn positions(&self, wallet: WalletAddress) -> Result<CarryAccount, EarnProviderError>;
}

#[async_trait]
pub trait EarnActionGateway: Send + Sync {
    async fn prepare_deposit(
        &self,
        command: &EarnDepositCommand,
    ) -> Result<CarryDepositPlan, EarnProviderError>;
}
