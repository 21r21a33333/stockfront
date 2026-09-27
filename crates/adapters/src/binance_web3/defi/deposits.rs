use async_trait::async_trait;
use stockfront_core::carry::{
    CarryDepositPlan, EarnActionGateway, EarnDepositCommand, EarnProviderError,
};

use super::{
    super::client::BinanceWeb3Client, gateway::DepositPlanError, mapping::ProviderCarryDepositPlan,
};

#[async_trait]
impl EarnActionGateway for BinanceWeb3Client {
    async fn prepare_deposit(
        &self,
        command: &EarnDepositCommand,
    ) -> Result<CarryDepositPlan, EarnProviderError> {
        let plan = self
            .build_deposit_plan(
                command.wallet,
                command.investment_id.clone(),
                command.asset,
                command.amount,
            )
            .await
            .map_err(EarnProviderError::from)?;
        CarryDepositPlan::try_from(ProviderCarryDepositPlan(plan))
    }
}

impl From<DepositPlanError> for EarnProviderError {
    fn from(error: DepositPlanError) -> Self {
        match error {
            DepositPlanError::Request(error) => error.into(),
            DepositPlanError::InvalidData => Self::InvalidData,
        }
    }
}
