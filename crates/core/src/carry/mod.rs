mod model;
mod ports;
mod service;

pub use model::{
    CarryAccount, CarryBalanceChange, CarryDepositPlan, CarryDepositPreview, CarryOpportunity,
    CarryOpportunityQuery, CarryOpportunityReason, CarryPosition, CarryQualification,
    CarryTransactionKind, EarnDepositCommand, EarnOpportunity, EarnRateType,
    PrepareCarryDepositRequest, PreparedCarryDeposit, PreparedCarryTransaction,
};
pub use ports::{EarnActionGateway, EarnProviderError, EarnReader};
pub use service::{CarryPolicy, CarryService, CarryServiceError};
