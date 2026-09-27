use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;

use crate::carry::EarnOpportunity;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum EarnOpportunityProviderError {
    #[error("earn opportunities are temporarily unavailable")]
    Unavailable,
    #[error("the earn opportunity provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait EarnOpportunityReader: Send + Sync {
    async fn opportunities(
        &self,
        asset: Address,
    ) -> Result<Vec<EarnOpportunity>, EarnOpportunityProviderError>;
}
