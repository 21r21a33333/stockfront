use alloy_primitives::{Address, B256};
use async_trait::async_trait;
use thiserror::Error;

use crate::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingMarket, BorrowingTransactionStatus, PrepareBorrowRequest,
        PrepareCollateralRequest, PrepareRepaymentRequest, PrepareSupplyRequest,
        PrepareWithdrawalRequest, PreparedBorrowingAction,
    },
    bstock_market::BstockId,
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BorrowingProviderError {
    #[error("the borrowing provider is temporarily unavailable")]
    Unavailable,
    #[error("the borrowing provider rejected the request")]
    Rejected,
    #[error("the borrowing provider returned invalid data")]
    InvalidData,
    #[error("the borrowing market has insufficient liquidity")]
    InsufficientBorrowLiquidity,
    #[error("repayment approval is required")]
    RepaymentApprovalRequired,
    #[error("the wallet has insufficient supplied balance")]
    InsufficientSuppliedBalance,
    #[error("the borrowing transaction was not found")]
    TransactionNotFound,
}

#[async_trait]
pub trait BorrowingActionGateway: Send + Sync {
    async fn prepare_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn prepare_collateral(
        &self,
        request: &PrepareCollateralRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn prepare_borrow(
        &self,
        request: &PrepareBorrowRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn prepare_repayment_approval(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn prepare_repayment(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn prepare_withdrawal(
        &self,
        request: &PrepareWithdrawalRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError>;

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<BorrowingTransactionStatus, BorrowingProviderError>;
}

#[async_trait]
pub trait BorrowingReader: Send + Sync {
    async fn read_markets(
        &self,
        collateral_assets: &[BstockId],
        debt_tokens: &[Address],
    ) -> Result<Vec<BorrowingMarket>, BorrowingProviderError>;

    async fn read_account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingProviderError>;
}
