#[path = "support/borrowing.rs"]
mod borrowing_support;
mod support;

use std::{env, error::Error, io};

use alloy::{sol, sol_types::SolCall};
use alloy_primitives::{Address, B256, U256, address};
use stockfront_core::bstock_borrowing::{
    BorrowingActionGateway, BorrowingProviderError, BorrowingTransactionKind, PrepareSupplyRequest,
};
use stockfront_core::wallet_portfolio::WalletAddress;

use borrowing_support::live_venus_adapter;

const TESLA_BSTOCK: Address = address!("5b1910eaad6450e50f816082aa078c41f10c292f");
const VENUS_TESLA_MARKET: Address = address!("97421799419eb782628e73e7220d8e0a207469a3");

sol! {
    function approve(address spender, uint256 amount) external returns (bool);
    function mint(uint256 amount) external returns (uint256);
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and an active Venus bStock investment"]
async fn prepares_live_bstock_supply_from_binance() -> Result<(), Box<dyn Error>> {
    let supply = live_venus_adapter(true)
        .await?
        .prepare_supply(&PrepareSupplyRequest {
            wallet: test_wallet()?,
            collateral_token: TESLA_BSTOCK,
            amount: "0.01".parse()?,
        })
        .await?;

    let [approval, deposit] = supply.transactions.as_slice() else {
        return Err(io::Error::other("expected approval followed by deposit").into());
    };
    assert_eq!(approval.kind, BorrowingTransactionKind::Approve);
    assert_eq!(approval.to, TESLA_BSTOCK);
    let approval_call = approveCall::abi_decode(&approval.data)?;
    assert_eq!(approval_call.spender, VENUS_TESLA_MARKET);
    assert_eq!(approval_call.amount, U256::from(10_000_000_000_000_000_u64));

    assert_eq!(deposit.kind, BorrowingTransactionKind::Deposit);
    assert_eq!(deposit.to, VENUS_TESLA_MARKET);
    let deposit_call = mintCall::abi_decode(&deposit.data)?;
    assert_eq!(deposit_call.amount, U256::from(10_000_000_000_000_000_u64));

    for transaction in &supply.transactions {
        assert_eq!(transaction.value, U256::ZERO);
        assert!(
            transaction
                .gas_limit
                .is_some_and(|value| value > U256::ZERO)
        );
        assert!(
            transaction
                .gas_price
                .is_some_and(|value| value > U256::ZERO)
        );
        assert!(
            transaction
                .max_priority_fee_per_gas
                .is_some_and(|value| value > U256::ZERO)
        );
        assert!(
            transaction
                .max_fee_per_gas
                .is_some_and(|value| value > U256::ZERO)
        );
    }

    let fee = supply
        .preview
        .estimated_network_fee
        .ok_or_else(|| io::Error::other("network fee missing from supply preview"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert!(fee.value_usd.is_some_and(|value| value.is_sign_positive()));
    assert_eq!(supply.preview.health_factor_before, None);
    assert_eq!(supply.preview.health_factor_after, None);
    assert!(supply.preview.warnings.is_empty());
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
    let error = live_venus_adapter(true)
        .await?
        .transaction_status(B256::ZERO)
        .await
        .expect_err("zero transaction hash unexpectedly exists");

    assert!(matches!(error, BorrowingProviderError::TransactionNotFound));
    Ok(())
}
