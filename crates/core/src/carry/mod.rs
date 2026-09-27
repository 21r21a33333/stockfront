mod model;
mod ports;
mod service;

pub use model::{
    CarryBalanceChange, CarryDepositPlan, CarryDepositPreview, CarryOpportunity,
    CarryOpportunityQuery, CarryOpportunityReason, CarryQualification, CarryTransactionKind,
    EarnDepositCommand, EarnOpportunity, EarnRateType, PrepareCarryDepositRequest,
    PreparedCarryDeposit, PreparedCarryTransaction,
};
pub use ports::{EarnActionGateway, EarnOpportunityReader, EarnProviderError};
pub use service::{CarryPolicy, CarryService, CarryServiceError};
