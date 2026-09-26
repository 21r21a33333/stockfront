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
#[ignore = "requires live Venus and BNB Chain provider access"]
async fn reads_live_bstock_borrowing_market() -> Result<(), Box<dyn Error>> {
    let reader = live_venus_adapter(false).await?;
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

    let market = &markets[0];
    assert!(market.oracle_valid);
    assert!(market.supply_available);
    assert!(market.borrow_available);
    assert!(market.remaining_supply_capacity.is_sign_positive());
    assert!(market.available_borrow_liquidity.is_sign_positive());
    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and live provider access"]
async fn reads_live_wallet_borrowing_account() -> Result<(), Box<dyn Error>> {
    let reader = live_venus_adapter(true).await?;
    let wallet_address: Address = env::var("STOCKFRONT_TEST_WALLET")?.parse()?;
    let wallet = WalletAddress::try_from(wallet_address)?;
    let account = reader
        .read_account(wallet)
        .await
        .map_err(|error| io::Error::other(format!("account read failed: {error}")))?;
    assert_eq!(account.chain_id, BNB_CHAIN_ID);
    assert!(account.observed_at_ms > 0);
    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and a STOCKFRONT_TEST_WALLET with supplied Venus collateral"]
async fn prepares_live_collateral_activation_and_borrow() -> Result<(), Box<dyn Error>> {
    let adapter = live_venus_adapter(true).await?;
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
