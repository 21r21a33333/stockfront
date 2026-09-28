use async_trait::async_trait;
use stockfront_core::liquidity::{
    LiquidityActionGateway, LiquidityEntryCommand, LiquidityProviderError, PreparedLiquidityEntry,
};

use super::{
    super::client::BinanceWeb3Client, gateway::DefiPlanError, mapping::ProviderLiquidityEntryPlan,
};

#[async_trait]
impl LiquidityActionGateway for BinanceWeb3Client {
    async fn prepare_entry(
        &self,
        command: &LiquidityEntryCommand,
    ) -> Result<PreparedLiquidityEntry, LiquidityProviderError> {
        let plan = self
            .build_lp_entry_plan(
                command.wallet,
                command.investment_id.clone(),
                command.input.token,
                command.input.amount,
                command.price_range_percent,
                command.slippage_bps,
            )
            .await
            .map_err(LiquidityProviderError::from)?;
        PreparedLiquidityEntry::try_from(ProviderLiquidityEntryPlan(plan))
    }
}

impl From<DefiPlanError> for LiquidityProviderError {
    fn from(error: DefiPlanError) -> Self {
        match error {
            DefiPlanError::Request(error) => error.into(),
            DefiPlanError::InvalidData => Self::InvalidData,
        }
    }
}
