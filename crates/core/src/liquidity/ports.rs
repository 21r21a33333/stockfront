use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;

use crate::liquidity::{LiquidityEntryCommand, LiquidityOpportunity, PreparedLiquidityEntry};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum LiquidityProviderError {
    #[error("the liquidity provider is temporarily unavailable")]
    Unavailable,
    #[error("the liquidity provider rejected the request")]
    Rejected,
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

#[async_trait]
pub trait LiquidityActionGateway: Send + Sync {
    async fn prepare_entry(
        &self,
        command: &LiquidityEntryCommand,
    ) -> Result<PreparedLiquidityEntry, LiquidityProviderError>;
}
