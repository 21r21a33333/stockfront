use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;

use crate::{
    bstock_borrowing::{BorrowingAccount, BorrowingMarket},
    bstock_market::BstockId,
    wallet_portfolio::WalletAddress,
};

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BorrowingReadError {
    #[error("borrowing data is temporarily unavailable")]
    Unavailable,
    #[error("the borrowing provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait BorrowingReader: Send + Sync {
    async fn read_markets(
        &self,
        collateral_assets: &[BstockId],
        debt_tokens: &[Address],
    ) -> Result<Vec<BorrowingMarket>, BorrowingReadError>;

    async fn read_account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingReadError>;
}
