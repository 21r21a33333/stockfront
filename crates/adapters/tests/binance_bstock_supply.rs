mod support;

use std::{env, error::Error};

use alloy_primitives::{Address, B256, address};
use stockfront_core::bstock_borrowing::{
    BorrowingActionGateway, BorrowingProviderError, PrepareSupplyRequest, SupplyTransactionKind,
};
use stockfront_core::wallet_portfolio::WalletAddress;

use support::live_client;

const NVIDIA_BSTOCK: Address = address!("02fca66c1d1afb4e2a7884261eb00f63598a7436");

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and an active Venus bStock investment"]
async fn prepares_live_bstock_supply_from_binance() -> Result<(), Box<dyn Error>> {
    let supply = live_client()?
        .prepare_supply(&PrepareSupplyRequest {
            wallet: test_wallet()?,
            collateral_token: NVIDIA_BSTOCK,
            amount: "0.001".parse()?,
        })
        .await?;

    assert!(!supply.transactions.is_empty());
    assert!(
        supply
            .transactions
            .iter()
            .all(|transaction| !transaction.data.is_empty())
    );
    assert_eq!(
        supply
            .transactions
            .last()
            .map(|transaction| transaction.kind),
        Some(SupplyTransactionKind::Deposit)
    );
    Ok(())
}

fn test_wallet() -> Result<WalletAddress, Box<dyn Error>> {
    Ok(WalletAddress::try_from(
        env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?,
    )?)
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn reports_unknown_live_borrowing_transaction() -> Result<(), Box<dyn Error>> {
    let error = live_client()?
        .transaction_status(B256::ZERO)
        .await
        .expect_err("zero transaction hash unexpectedly exists");

    assert!(matches!(error, BorrowingProviderError::TransactionNotFound));
    Ok(())
}
