use async_trait::async_trait;
use stockfront_core::liquidity::{
    LiquidityActionGateway, LiquidityEntryCommand, LiquidityProviderError,
    LiquidityWithdrawalCommand, PrepareLiquidityClaimRequest, PreparedLiquidityAction,
    PreparedLiquidityEntry,
};

use super::{
    super::client::BinanceWeb3Client,
    gateway::DefiPlanError,
    mapping::{
        ProviderLiquidityClaimPlan, ProviderLiquidityEntryPlan, ProviderLiquidityWithdrawalPlan,
    },
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

    async fn prepare_claim(
        &self,
        request: &PrepareLiquidityClaimRequest,
    ) -> Result<PreparedLiquidityAction, LiquidityProviderError> {
        let plan = self
            .build_lp_fee_claim_plan(request)
            .await
            .map_err(LiquidityProviderError::from)?;
        PreparedLiquidityAction::try_from(ProviderLiquidityClaimPlan(plan))
    }

    async fn prepare_withdrawal(
        &self,
        command: &LiquidityWithdrawalCommand,
    ) -> Result<PreparedLiquidityAction, LiquidityProviderError> {
        let plan = self
            .build_lp_withdrawal_plan(command)
            .await
            .map_err(LiquidityProviderError::from)?;
        PreparedLiquidityAction::try_from(ProviderLiquidityWithdrawalPlan(plan))
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
