#[path = "support/borrowing.rs"]
mod borrowing_support;
mod support;

use std::{env, error::Error, io};

use alloy_primitives::{Address, address};
use stockfront_core::{
    bstock_borrowing::{
        BorrowingActionGateway, BorrowingReader, BorrowingTransactionKind, PrepareBorrowRequest,
        PrepareCollateralRequest,
    },
    bstock_market::{BNB_CHAIN_ID, BstockId},
    wallet_portfolio::WalletAddress,
};

use borrowing_support::live_venus_adapter;

const TSLAB: alloy_primitives::Address = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and live provider access"]
async fn reads_bstock_market_and_empty_pooled_account() -> Result<(), Box<dyn Error>> {
    let reader = live_venus_adapter().await?;
    let markets = reader
        .read_markets(
            &[BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: TSLAB,
            }],
            &[USDT],
        )
        .await
        .map_err(|error| io::Error::other(format!("market read failed: {error}")))?;
    assert_eq!(markets.len(), 1);
    assert_eq!(markets[0].collateral.contract, TSLAB);
    assert_eq!(markets[0].debt.contract, USDT);
    assert!(markets[0].observed_at_ms > 0);

    let wallet = WalletAddress::try_from(address!("0000000000000000000000000000000000000001"))?;
    let account = reader
        .read_account(wallet)
        .await
        .map_err(|error| io::Error::other(format!("account read failed: {error}")))?;
    assert!(account.positions.is_empty());
    assert!(account.health_factor.is_none());
    assert_eq!(account.chain_id, BNB_CHAIN_ID);
    assert!(account.observed_at_ms > 0);
    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and a STOCKFRONT_TEST_WALLET with supplied Venus collateral"]
async fn prepares_live_collateral_activation_and_borrow() -> Result<(), Box<dyn Error>> {
    let adapter = live_venus_adapter().await?;
    let wallet_address: Address = env::var("STOCKFRONT_TEST_WALLET")?.parse()?;
    let wallet = WalletAddress::try_from(wallet_address)?;

    let collateral = adapter
        .prepare_collateral(&PrepareCollateralRequest {
            wallet,
            collateral_token: TSLAB,
        })
        .await?;
    assert!(
        collateral.transactions.is_empty()
            || collateral
                .transactions
                .iter()
                .all(|transaction| transaction.kind == BorrowingTransactionKind::EnableCollateral)
    );

    let borrow = adapter
        .prepare_borrow(&PrepareBorrowRequest {
            wallet,
            debt_token: USDT,
            amount: "0.01".parse()?,
        })
        .await?;
    assert_eq!(borrow.transactions.len(), 1);
    assert_eq!(
        borrow.transactions[0].kind,
        BorrowingTransactionKind::Borrow
    );
    Ok(())
}
