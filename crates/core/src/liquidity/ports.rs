use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;

use crate::liquidity::LiquidityOpportunity;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum LiquidityProviderError {
    #[error("liquidity opportunities are temporarily unavailable")]
    Unavailable,
    #[error("the liquidity opportunity provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait LiquidityOpportunityReader: Send + Sync {
    async fn opportunities(
        &self,
        bstock_token: Address,
    ) -> Result<Vec<LiquidityOpportunity>, LiquidityProviderError>;
}
