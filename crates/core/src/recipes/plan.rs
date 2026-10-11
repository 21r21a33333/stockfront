//! Provider-independent output produced by the recipe compiler.
//!
//! A logical plan says what each node needs to do, but it does not contain
//! calldata, quotes, or provider IDs. Those values change over time and belong
//! in preview and prepare services.
//!
//! Example:
//!
//! ```text
//! step 1: trade 6 USDT from the wallet into TSLAB
//! step 2: use step 1's `buy` output as the next action's input
//! ```
//!
//! Step 2 cannot be prepared safely until step 1 has executed and its real
//! output is known. The `NodeOutput` reference keeps that dependency explicit.

use alloy_primitives::{Address, U256};
use uuid::Uuid;

use crate::bstock_borrowing::BorrowingVenue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SuppliedPositionRef {
    pub venue: BorrowingVenue,
    pub pool: Address,
    pub token: Address,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EarnPositionRef {
    pub investment_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BorrowedPositionRef {
    pub venue: BorrowingVenue,
    pub pool: Address,
    pub token: Address,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AssetRef {
    Wallet {
        token: Address,
        amount: U256,
    },
    NodeOutput {
        node_id: Uuid,
        port: String,
        token: Address,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum RecipeAction {
    Trade {
        sell: AssetRef,
        buy_token: Address,
    },
    Supply {
        asset: AssetRef,
        venue: BorrowingVenue,
    },
    Withdraw {
        position: SuppliedPositionRef,
    },
    Borrow {
        collateral: SuppliedPositionRef,
        debt_token: Address,
        /// `None` asks for the safe maximum, resolved against the chain when
        /// the collateral behind it actually exists.
        raw_amount: Option<U256>,
    },
    Repay {
        debt: BorrowedPositionRef,
    },
    EarnDeposit {
        asset: AssetRef,
        collateral_token: Address,
        investment_id: String,
    },
    EarnWithdraw {
        position: EarnPositionRef,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LogicalStep {
    pub node_id: Uuid,
    pub dependencies: Vec<Uuid>,
    pub action: RecipeAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LogicalPlan {
    pub steps: Vec<LogicalStep>,
}
