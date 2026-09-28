use async_trait::async_trait;
use stockfront_core::carry::{
    CarryActionPlan, EarnActionGateway, EarnDepositCommand, EarnProviderError,
    EarnRedemptionClaimCommand, EarnWithdrawalCommand,
};

use super::{
    super::client::BinanceWeb3Client,
    gateway::DefiPlanError,
    mapping::{ProviderCarryClaimPlan, ProviderCarryDepositPlan, ProviderCarryWithdrawalPlan},
};

#[async_trait]
impl EarnActionGateway for BinanceWeb3Client {
    async fn prepare_deposit(
        &self,
        command: &EarnDepositCommand,
    ) -> Result<CarryActionPlan, EarnProviderError> {
        let plan = self
            .build_deposit_plan(
                command.wallet,
                command.investment_id.clone(),
                command.asset,
                command.amount,
            )
            .await
            .map_err(EarnProviderError::from)?;
        CarryActionPlan::try_from(ProviderCarryDepositPlan(plan))
    }

    async fn prepare_withdrawal(
        &self,
        command: &EarnWithdrawalCommand,
    ) -> Result<CarryActionPlan, EarnProviderError> {
        let plan = self
            .build_redeem_plan(
                command.wallet,
                command.investment_id.clone(),
                &command.amount,
            )
            .await
            .map_err(EarnProviderError::from)?;
        CarryActionPlan::try_from(ProviderCarryWithdrawalPlan(plan))
    }

    async fn prepare_redemption_claim(
        &self,
        command: &EarnRedemptionClaimCommand,
    ) -> Result<CarryActionPlan, EarnProviderError> {
        let plan = self
            .build_redemption_claim_plan(
                command.wallet,
                command.investment_id.clone(),
                command.redemption_id.clone(),
            )
            .await
            .map_err(EarnProviderError::from)?;
        CarryActionPlan::try_from(ProviderCarryClaimPlan(plan))
    }
}

impl From<DefiPlanError> for EarnProviderError {
    fn from(error: DefiPlanError) -> Self {
        match error {
            DefiPlanError::Request(error) => error.into(),
            DefiPlanError::InvalidData => Self::InvalidData,
        }
    }
}
