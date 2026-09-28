mod model;
mod ports;
mod service;

pub use model::{
    CarryAccount, CarryActionPlan, CarryActionPreview, CarryBalanceChange, CarryOpportunity,
    CarryOpportunityQuery, CarryOpportunityReason, CarryPosition, CarryQualification,
    CarryRedemption, CarryTransactionKind, CarryWithdrawalAmount, EarnDepositCommand,
    EarnOpportunity, EarnRateType, EarnRedemptionClaimCommand, EarnWithdrawalCommand,
    PrepareCarryDepositRequest, PrepareCarryRedemptionClaimRequest, PrepareCarryWithdrawalRequest,
    PreparedCarryDeposit, PreparedCarryTransaction, RedemptionDelay,
};
pub use ports::{EarnActionGateway, EarnProviderError, EarnReader};
pub use service::{CarryPolicy, CarryService, CarryServiceError};
