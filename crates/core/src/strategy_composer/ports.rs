use async_trait::async_trait;
use thiserror::Error;

use super::{ComposeContext, StrategyDraft};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum StrategyModelError {
    #[error("the strategy model is rate limited")]
    RateLimited,
    #[error("the strategy model is temporarily unavailable")]
    Unavailable,
    #[error("the strategy model returned an invalid response")]
    InvalidResponse,
}

#[async_trait]
pub trait StrategyModel: Send + Sync {
    async fn draft(&self, context: &ComposeContext) -> Result<StrategyDraft, StrategyModelError>;
}
