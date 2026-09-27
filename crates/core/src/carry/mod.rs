mod model;
mod ports;
mod service;

pub use model::{
    CarryOpportunity, CarryOpportunityQuery, CarryOpportunityReason, CarryQualification,
    EarnOpportunity, EarnRateType,
};
pub use ports::{EarnOpportunityProviderError, EarnOpportunityReader};
pub use service::{CarryOpportunityService, CarryOpportunityServiceError, CarryPolicy};
