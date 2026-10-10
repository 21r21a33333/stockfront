#[path = "support/borrowing.rs"]
mod borrowing_support;
#[path = "support/erc20_allowance.rs"]
mod erc20_allowance_support;
#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;
mod support;

use std::{env, error::Error, io, time::Duration};

use alloy::{sol, sol_types::SolCall};
use alloy_primitives::{Address, B256, U256, address};
use rust_decimal::Decimal;
use stockfront_core::bstock_borrowing::{
    BorrowingActionGateway, BorrowingProviderError, BorrowingReader, BorrowingTransactionKind,
    PrepareSupplyRequest, PreparedBorrowingAction, PreparedBorrowingTransaction,
};
use stockfront_core::wallet_portfolio::WalletAddress;

use borrowing_support::live_venus_adapter;
use erc20_allowance_support::token_allowance;
use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};

const TESLA_BSTOCK: Address = address!("5b1910eaad6450e50f816082aa078c41f10c292f");
const VENUS_TESLA_MARKET: Address = address!("97421799419eb782628e73e7220d8e0a207469a3");
const SUPPLY_AMOUNT: &str = "0.01";
const SUPPLY_AMOUNT_RAW: u64 = 10_000_000_000_000_000;
const POSITION_ATTEMPTS: usize = 30;
const POSITION_POLL_INTERVAL: Duration = Duration::from_secs(2);

sol! {
    function approve(address spender, uint256 amount) external returns (bool);
    function mint(uint256 amount) external returns (uint256);
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and an active Venus bStock investment"]
async fn prepares_live_bstock_supply() -> Result<(), Box<dyn Error>> {
    let supply = live_venus_adapter(true)
        .await?
        .prepare_supply(&PrepareSupplyRequest {
            wallet: test_wallet()?,
            collateral_token: TESLA_BSTOCK,
            amount: SUPPLY_AMOUNT.parse()?,
        })
        .await?;

    validated_supply_transactions(&supply)?;
    Ok(())
}

#[tokio::test]
#[ignore = "supplies 0.01 TSLAB to Venus on BSC mainnet and spends BNB for gas"]
async fn supplies_tesla_bstock_to_venus_on_bsc() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let wallet = context.wallet.as_address();
    let amount = U256::from(SUPPLY_AMOUNT_RAW);
    let starting_bstock = token_balance(&context.provider, TESLA_BSTOCK, wallet).await?;
    if starting_bstock < amount {
        return Err(io::Error::other("wallet needs at least 0.01 TSLAB").into());
    }
    let starting_vtoken = token_balance(&context.provider, VENUS_TESLA_MARKET, wallet).await?;

    let adapter = live_venus_adapter(true).await?;
    let supply = adapter
        .prepare_supply(&PrepareSupplyRequest {
            wallet: context.wallet,
            collateral_token: TESLA_BSTOCK,
            amount: SUPPLY_AMOUNT.parse()?,
        })
        .await?;
    let (approval, deposit) = validated_supply_transactions(&supply)?;

    if let Some(approval) = approval {
        send_transaction(
            &context.provider,
            approval.to,
            approval.data.clone(),
            approval.value,
        )
        .await?;
        let allowance =
            token_allowance(&context.provider, TESLA_BSTOCK, wallet, VENUS_TESLA_MARKET).await?;
        assert_eq!(allowance, amount);
    }

    send_transaction(
        &context.provider,
        deposit.to,
        deposit.data.clone(),
        deposit.value,
    )
    .await?;

    let final_bstock = token_balance(&context.provider, TESLA_BSTOCK, wallet).await?;
    let expected_bstock = starting_bstock
        .checked_sub(amount)
        .ok_or_else(|| io::Error::other("TSLAB balance subtraction underflowed"))?;
    assert_eq!(final_bstock, expected_bstock);
    let final_vtoken = token_balance(&context.provider, VENUS_TESLA_MARKET, wallet).await?;
    assert!(final_vtoken > starting_vtoken);
    if approval.is_some() {
        let remaining_allowance =
            token_allowance(&context.provider, TESLA_BSTOCK, wallet, VENUS_TESLA_MARKET).await?;
        assert_eq!(remaining_allowance, U256::ZERO);
    }

    wait_for_supplied_position(&adapter, context.wallet).await
}

type SupplyTransactions<'a> = (
    Option<&'a PreparedBorrowingTransaction>,
    &'a PreparedBorrowingTransaction,
);

fn validated_supply_transactions(
    supply: &PreparedBorrowingAction,
) -> Result<SupplyTransactions<'_>, Box<dyn Error>> {
    let (approval, deposit) = match supply.transactions.as_slice() {
        [deposit] => (None, deposit),
        [approval, deposit] => (Some(approval), deposit),
        _ => {
            return Err(
                io::Error::other("expected a deposit optionally preceded by an approval").into(),
            );
        }
    };
    if let Some(approval) = approval {
        assert_eq!(approval.kind, BorrowingTransactionKind::Approve);
        assert_eq!(approval.to, TESLA_BSTOCK);
        let approval_call = approveCall::abi_decode(&approval.data)?;
        assert_eq!(approval_call.spender, VENUS_TESLA_MARKET);
        assert_eq!(approval_call.amount, U256::from(SUPPLY_AMOUNT_RAW));
    }

    assert_eq!(deposit.kind, BorrowingTransactionKind::Deposit);
    assert_eq!(deposit.to, VENUS_TESLA_MARKET);
    let deposit_call = mintCall::abi_decode(&deposit.data)?;
    assert_eq!(deposit_call.amount, U256::from(SUPPLY_AMOUNT_RAW));

    // Binance prices only the leading transaction; a deposit that follows an
    // approval cannot be simulated until that approval is mined.
    let priced = approval.unwrap_or(deposit);
    assert!(priced.gas_limit.is_some_and(|value| value > U256::ZERO));
    assert!(priced.gas_price.is_some_and(|value| value > U256::ZERO));
    if approval.is_some() {
        assert_eq!(deposit.gas_limit, None);
        assert_eq!(deposit.gas_price, None);
    }
    for transaction in &supply.transactions {
        assert_eq!(transaction.value, U256::ZERO);
        assert_eq!(transaction.max_priority_fee_per_gas, None);
        assert_eq!(transaction.max_fee_per_gas, None);
    }

    let fee = supply
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("network fee missing from supply preview"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert_eq!(fee.value_usd, None);
    assert_eq!(supply.preview.health_factor_before, None);
    assert_eq!(supply.preview.health_factor_after, None);
    assert!(supply.preview.warnings.is_empty());
    Ok((approval, deposit))
}

async fn wait_for_supplied_position(
    reader: &impl BorrowingReader,
    wallet: WalletAddress,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let account = reader.read_account(wallet).await?;
        let indexed = account.positions.iter().any(|position| {
            position
                .supplied
                .iter()
                .any(|asset| asset.token.contract == TESLA_BSTOCK && asset.amount > Decimal::ZERO)
        });
        if indexed {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other("Binance did not index the Venus TSLAB supply in time").into())
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
