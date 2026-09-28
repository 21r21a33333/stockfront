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
use alloy_primitives::{Address, B256, U256, address, utils::parse_units};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingActionGateway, BorrowingReader, BorrowingTransactionKind,
        PositionAmount, PrepareBorrowRequest, PrepareCollateralRequest, PrepareRepaymentRequest,
        PrepareWithdrawalRequest, PreparedBorrowingAction, PreparedBorrowingTransaction,
    },
    bstock_market::{BNB_CHAIN_ID, BstockId},
    wallet_portfolio::WalletAddress,
};

use borrowing_support::live_venus_adapter;
use erc20_allowance_support::token_allowance;
use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};

const TSLAB: alloy_primitives::Address = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");
const VENUS_TSLAB: alloy_primitives::Address = address!("97421799419Eb782628e73e7220d8E0A207469a3");
const VENUS_COMPTROLLER: alloy_primitives::Address =
    address!("fD36E2c2a6789Db23113685031d7F16329158384");
const VENUS_USDT: alloy_primitives::Address = address!("fD5840Cd36d94D7229439859C0112a4185BC0255");
const BORROW_AMOUNT: &str = "1";
const BORROW_AMOUNT_RAW: u64 = 1_000_000_000_000_000_000;
const POSITION_ATTEMPTS: usize = 30;
const POSITION_POLL_INTERVAL: Duration = Duration::from_secs(2);

sol! {
    #[sol(rpc)]
    interface TestVenusComptroller {
        function checkMembership(address account, address vToken) external view returns (bool);
        function enterMarkets(address[] vTokens) external returns (uint256[]);
    }

    #[sol(rpc)]
    interface TestVenusDebtMarket {
        function borrow(uint256 amount) external returns (uint256);
        function borrowBalanceCurrent(address account) external returns (uint256);
        function borrowBalanceStored(address account) external view returns (uint256);
        function repayBorrow(uint256 amount) external returns (uint256);
    }

    #[sol(rpc)]
    interface TestVenusCollateralMarket {
        function redeem(uint256 redeemTokens) external returns (uint256);
    }

    function approve(address spender, uint256 amount) external returns (bool);
}

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
#[ignore = "enables supplied TSLAB as Venus collateral on BSC mainnet and spends BNB for gas"]
async fn enables_live_tesla_bstock_collateral_on_venus() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let wallet = context.wallet.as_address();
    let comptroller = TestVenusComptroller::new(VENUS_COMPTROLLER, &context.provider);
    let enabled_before = comptroller
        .checkMembership(wallet, VENUS_TSLAB)
        .call()
        .await?;
    let adapter = live_venus_adapter(true).await?;
    let collateral = adapter
        .prepare_collateral(&PrepareCollateralRequest {
            wallet: context.wallet,
            collateral_token: TSLAB,
        })
        .await?;

    if enabled_before {
        assert!(collateral.transactions.is_empty());
        return Ok(());
    }

    let [transaction] = collateral.transactions.as_slice() else {
        return Err(io::Error::other("expected one collateral activation transaction").into());
    };
    assert_eq!(transaction.kind, BorrowingTransactionKind::EnableCollateral);
    assert_eq!(transaction.to, VENUS_COMPTROLLER);
    assert_eq!(transaction.value, U256::ZERO);
    let call = TestVenusComptroller::enterMarketsCall::abi_decode(&transaction.data)?;
    assert_eq!(call.vTokens, vec![VENUS_TSLAB]);
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
    assert_eq!(transaction.max_priority_fee_per_gas, None);
    assert_eq!(transaction.max_fee_per_gas, None);
    let fee = collateral
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("collateral network fee is missing"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert_eq!(fee.value_usd, None);
    assert_eq!(collateral.preview.health_factor_before, None);
    assert_eq!(collateral.preview.health_factor_after, None);
    assert!(collateral.preview.warnings.is_empty());

    send_transaction(
        &context.provider,
        transaction.to,
        transaction.data.clone(),
        transaction.value,
    )
    .await?;
    assert!(
        comptroller
            .checkMembership(wallet, VENUS_TSLAB)
            .call()
            .await?
    );
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
            amount: BORROW_AMOUNT.parse()?,
        })
        .await?;
    validated_borrow_transaction(&borrow)?;
    Ok(())
}

#[tokio::test]
#[ignore = "borrows 1 USDT from Venus on BSC mainnet and creates live debt"]
async fn borrows_live_usdt_against_tesla_bstock_on_venus() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let wallet = context.wallet.as_address();
    let amount = U256::from(BORROW_AMOUNT_RAW);
    let debt_market = TestVenusDebtMarket::new(VENUS_USDT, &context.provider);
    let starting_usdt = token_balance(&context.provider, USDT, wallet).await?;
    let starting_debt = debt_market.borrowBalanceStored(wallet).call().await?;

    let adapter = live_venus_adapter(true).await?;
    let starting_indexed_debt = indexed_usdt_debt(&adapter, context.wallet).await?;
    let borrow = adapter
        .prepare_borrow(&PrepareBorrowRequest {
            wallet: context.wallet,
            debt_token: USDT,
            amount: BORROW_AMOUNT.parse()?,
        })
        .await?;
    let transaction = validated_borrow_transaction(&borrow)?;

    let transaction_hash = send_transaction(
        &context.provider,
        transaction.to,
        transaction.data.clone(),
        transaction.value,
    )
    .await?;

    let final_usdt = token_balance(&context.provider, USDT, wallet).await?;
    let expected_usdt = starting_usdt
        .checked_add(amount)
        .ok_or_else(|| io::Error::other("USDT balance calculation overflowed"))?;
    assert_eq!(final_usdt, expected_usdt);

    let final_debt = debt_market.borrowBalanceStored(wallet).call().await?;
    let minimum_debt = starting_debt
        .checked_add(amount)
        .ok_or_else(|| io::Error::other("Venus debt calculation overflowed"))?;
    assert!(final_debt >= minimum_debt);

    wait_for_indexed_borrow(
        &adapter,
        context.wallet,
        starting_indexed_debt,
        transaction_hash,
    )
    .await
}

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and a STOCKFRONT_TEST_WALLET with live Venus debt"]
async fn prepares_full_live_usdt_repayment_approval() -> Result<(), Box<dyn Error>> {
    let wallet_address: Address = env::var("STOCKFRONT_TEST_WALLET")?.parse()?;
    let wallet = WalletAddress::try_from(wallet_address)?;
    let adapter = live_venus_adapter(true).await?;
    let indexed_debt = indexed_usdt_debt(&adapter, wallet).await?;
    if indexed_debt.is_zero() {
        return Err(io::Error::other("wallet has no Venus USDT debt to approve").into());
    }
    let indexed_debt_raw: U256 = parse_units(&indexed_debt.to_string(), 18)?.into();

    let approval = adapter
        .prepare_repayment_approval(&PrepareRepaymentRequest {
            wallet,
            debt_token: USDT,
            amount: PositionAmount::Full,
        })
        .await?;
    validated_repayment_approval(&approval, indexed_debt_raw)?;
    Ok(())
}

#[tokio::test]
#[ignore = "fully repays live Venus USDT debt on BSC mainnet and spends BNB for gas"]
async fn fully_repays_live_usdt_debt_on_venus() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let adapter = live_venus_adapter(true).await?;
    repay_live_usdt_debt(&context, &adapter).await
}

#[tokio::test]
#[ignore = "requires Binance Web3 credentials and a STOCKFRONT_TEST_WALLET with supplied Venus collateral"]
async fn prepares_full_live_tesla_bstock_withdrawal() -> Result<(), Box<dyn Error>> {
    let wallet_address: Address = env::var("STOCKFRONT_TEST_WALLET")?.parse()?;
    let wallet = WalletAddress::try_from(wallet_address)?;
    let adapter = live_venus_adapter(true).await?;
    let account = adapter
        .read_account(wallet)
        .await
        .map_err(|error| io::Error::other(format!("account preflight failed: {error}")))?;

    if !account.total_borrowed_usd.is_zero() {
        return Err(io::Error::other("wallet still has outstanding Venus debt").into());
    }
    if tslab_supply(&account)?.is_zero() {
        return Err(io::Error::other("wallet has no supplied Venus TSLAB to withdraw").into());
    }
    let withdrawal = adapter
        .prepare_withdrawal(&PrepareWithdrawalRequest {
            wallet,
            collateral_token: TSLAB,
            amount: PositionAmount::Full,
        })
        .await
        .map_err(|error| io::Error::other(format!("withdrawal preparation failed: {error}")))?;
    validated_full_withdrawal(&withdrawal)?;
    Ok(())
}

#[tokio::test]
#[ignore = "fully withdraws supplied TSLAB from Venus on BSC mainnet and spends BNB for gas"]
async fn fully_withdraws_live_tesla_bstock_from_venus() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let adapter = live_venus_adapter(true).await?;
    withdraw_live_tslab(&context, &adapter).await
}

#[tokio::test]
#[ignore = "fully repays Venus USDT debt and returns all supplied TSLAB to the wallet on BSC mainnet"]
async fn fully_repays_debt_and_returns_tesla_bstock_to_wallet() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let adapter = live_venus_adapter(true).await?;

    repay_live_usdt_debt(&context, &adapter).await?;
    withdraw_live_tslab(&context, &adapter).await
}

async fn repay_live_usdt_debt(
    context: &live_evm_support::LiveEvmContext,
    adapter: &stockfront_adapters::venus::VenusBorrowingAdapter,
) -> Result<(), Box<dyn Error>> {
    let wallet = context.wallet.as_address();
    let debt_market = TestVenusDebtMarket::new(VENUS_USDT, &context.provider);
    let starting_debt = debt_market.borrowBalanceCurrent(wallet).call().await?;
    if starting_debt == U256::ZERO {
        return Err(io::Error::other("wallet has no Venus USDT debt to repay").into());
    }

    let request = PrepareRepaymentRequest {
        wallet: context.wallet,
        debt_token: USDT,
        amount: PositionAmount::Full,
    };
    let approval = adapter.prepare_repayment_approval(&request).await?;
    let prepared_approval = validated_repayment_approval(&approval, starting_debt)?;
    let required_usdt = prepared_approval
        .as_ref()
        .map_or(starting_debt, |prepared| prepared.amount);
    let starting_usdt = token_balance(&context.provider, USDT, wallet).await?;
    if starting_usdt < required_usdt {
        return Err(io::Error::other(format!(
            "wallet needs {} more USDT base units to cover debt and accrued interest",
            required_usdt - starting_usdt
        ))
        .into());
    }
    if let Some(prepared) = prepared_approval {
        send_transaction(
            &context.provider,
            prepared.transaction.to,
            prepared.transaction.data.clone(),
            prepared.transaction.value,
        )
        .await?;
    }

    let debt_before_repayment = debt_market.borrowBalanceCurrent(wallet).call().await?;
    let allowance = token_allowance(&context.provider, USDT, wallet, VENUS_USDT).await?;
    assert!(allowance >= debt_before_repayment);
    let usdt_before_repayment = token_balance(&context.provider, USDT, wallet).await?;
    if usdt_before_repayment < debt_before_repayment {
        return Err(io::Error::other(format!(
            "wallet needs {} more USDT base units to repay accrued interest",
            debt_before_repayment - usdt_before_repayment
        ))
        .into());
    }

    let repayment = adapter.prepare_repayment(&request).await?;
    let transaction = validated_full_repayment(&repayment)?;
    let transaction_hash = send_transaction(
        &context.provider,
        transaction.to,
        transaction.data.clone(),
        transaction.value,
    )
    .await?;

    assert_eq!(
        debt_market.borrowBalanceStored(wallet).call().await?,
        U256::ZERO
    );
    let usdt_after_repayment = token_balance(&context.provider, USDT, wallet).await?;
    let spent = usdt_before_repayment
        .checked_sub(usdt_after_repayment)
        .ok_or_else(|| io::Error::other("USDT balance increased during repayment"))?;
    assert!(spent >= debt_before_repayment);
    assert!(spent <= allowance);

    wait_for_indexed_repayment(adapter, context.wallet, transaction_hash).await
}

async fn withdraw_live_tslab(
    context: &live_evm_support::LiveEvmContext,
    adapter: &stockfront_adapters::venus::VenusBorrowingAdapter,
) -> Result<(), Box<dyn Error>> {
    let wallet = context.wallet.as_address();
    let starting_vtoken_balance = token_balance(&context.provider, VENUS_TSLAB, wallet).await?;
    if starting_vtoken_balance == U256::ZERO {
        return Err(io::Error::other("wallet has no Venus TSLAB position to withdraw").into());
    }

    let account = adapter.read_account(context.wallet).await?;
    if !account.total_borrowed_usd.is_zero() {
        return Err(io::Error::other("wallet still has outstanding Venus debt").into());
    }
    if tslab_supply(&account)?.is_zero() {
        return Err(io::Error::other("Binance has not indexed the supplied TSLAB position").into());
    }

    let starting_tslab_balance = token_balance(&context.provider, TSLAB, wallet).await?;
    let withdrawal = adapter
        .prepare_withdrawal(&PrepareWithdrawalRequest {
            wallet: context.wallet,
            collateral_token: TSLAB,
            amount: PositionAmount::Full,
        })
        .await?;
    let (transaction, redeem_tokens) = validated_full_withdrawal(&withdrawal)?;
    assert_eq!(redeem_tokens, starting_vtoken_balance);

    let transaction_hash = send_transaction(
        &context.provider,
        transaction.to,
        transaction.data.clone(),
        transaction.value,
    )
    .await?;

    assert_eq!(
        token_balance(&context.provider, VENUS_TSLAB, wallet).await?,
        U256::ZERO
    );
    assert!(token_balance(&context.provider, TSLAB, wallet).await? > starting_tslab_balance);
    wait_for_indexed_withdrawal(adapter, context.wallet, transaction_hash).await
}

struct PreparedApproval<'action> {
    transaction: &'action PreparedBorrowingTransaction,
    amount: U256,
}

fn validated_repayment_approval(
    approval: &PreparedBorrowingAction,
    minimum_debt: U256,
) -> Result<Option<PreparedApproval<'_>>, Box<dyn Error>> {
    if approval.transactions.is_empty() {
        assert_eq!(approval.preview.estimated_network_fee, None);
        return Ok(None);
    }

    let [transaction] = approval.transactions.as_slice() else {
        return Err(io::Error::other("expected one repayment approval transaction").into());
    };
    assert_eq!(transaction.kind, BorrowingTransactionKind::Approve);
    assert_eq!(transaction.to, USDT);
    assert_eq!(transaction.value, U256::ZERO);
    let call = approveCall::abi_decode(&transaction.data)?;
    assert_eq!(call.spender, VENUS_USDT);
    assert!(call.amount >= minimum_debt);
    let maximum_approval = minimum_debt
        .checked_mul(U256::from(1_001))
        .and_then(|amount| amount.checked_div(U256::from(1_000)))
        .and_then(|amount| amount.checked_add(U256::from(1)))
        .ok_or_else(|| io::Error::other("repayment approval bound overflowed"))?;
    assert!(call.amount <= maximum_approval);

    let fee = approval
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("repayment approval network fee is missing"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert_eq!(fee.value_usd, None);
    assert_eq!(approval.preview.health_factor_before, None);
    assert_eq!(approval.preview.health_factor_after, None);
    assert!(approval.preview.warnings.is_empty());
    Ok(Some(PreparedApproval {
        transaction,
        amount: call.amount,
    }))
}

fn validated_full_repayment(
    repayment: &PreparedBorrowingAction,
) -> Result<&PreparedBorrowingTransaction, Box<dyn Error>> {
    let [transaction] = repayment.transactions.as_slice() else {
        return Err(io::Error::other("expected one full repayment transaction").into());
    };
    assert_eq!(transaction.kind, BorrowingTransactionKind::Repay);
    assert_eq!(transaction.to, VENUS_USDT);
    assert_eq!(transaction.value, U256::ZERO);
    let call = TestVenusDebtMarket::repayBorrowCall::abi_decode(&transaction.data)?;
    assert_eq!(call.amount, U256::MAX);
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

    let fee = repayment
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("repayment network fee is missing"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert_eq!(fee.value_usd, None);
    assert!(repayment.preview.warnings.is_empty());
    Ok(transaction)
}

fn validated_full_withdrawal(
    withdrawal: &PreparedBorrowingAction,
) -> Result<(&PreparedBorrowingTransaction, U256), Box<dyn Error>> {
    let [transaction] = withdrawal.transactions.as_slice() else {
        return Err(io::Error::other("expected one full withdrawal transaction").into());
    };
    assert_eq!(transaction.kind, BorrowingTransactionKind::Withdraw);
    assert_eq!(transaction.to, VENUS_TSLAB);
    assert_eq!(transaction.value, U256::ZERO);
    let call = TestVenusCollateralMarket::redeemCall::abi_decode(&transaction.data)?;
    assert!(call.redeemTokens > U256::ZERO);
    assert!(
        transaction
            .gas_limit
            .is_some_and(|value| value > U256::ZERO)
    );
    assert!(
        transaction
            .gas_price
            .is_some_and(|value| value > U256::ZERO)
            || transaction
                .max_fee_per_gas
                .is_some_and(|value| value > U256::ZERO)
    );

    let fee = withdrawal
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("withdrawal network fee is missing"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    if let Some(value_usd) = fee.value_usd {
        assert!(value_usd.is_sign_positive());
    }
    assert_eq!(withdrawal.preview.health_factor_before, None);
    assert_eq!(withdrawal.preview.health_factor_after, None);
    assert!(withdrawal.preview.warnings.is_empty());
    Ok((transaction, call.redeemTokens))
}

fn validated_borrow_transaction(
    borrow: &PreparedBorrowingAction,
) -> Result<&PreparedBorrowingTransaction, Box<dyn Error>> {
    let [transaction] = borrow.transactions.as_slice() else {
        return Err(io::Error::other("expected one borrow transaction").into());
    };
    assert_eq!(transaction.kind, BorrowingTransactionKind::Borrow);
    assert_eq!(transaction.to, VENUS_USDT);
    assert_eq!(transaction.value, U256::ZERO);
    let call = TestVenusDebtMarket::borrowCall::abi_decode(&transaction.data)?;
    assert_eq!(call.amount, U256::from(BORROW_AMOUNT_RAW));
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

    let fee = borrow
        .preview
        .estimated_network_fee
        .as_ref()
        .ok_or_else(|| io::Error::other("borrow network fee is missing"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert_eq!(fee.value_usd, None);
    let health_factor_after = borrow
        .preview
        .health_factor_after
        .ok_or_else(|| io::Error::other("projected health factor is missing"))?;
    assert!(health_factor_after > Decimal::ONE);
    if let Some(health_factor_before) = borrow.preview.health_factor_before {
        assert!(health_factor_after < health_factor_before);
    }
    assert!(borrow.preview.warnings.is_empty());
    Ok(transaction)
}

async fn indexed_usdt_debt(
    reader: &impl BorrowingReader,
    wallet: WalletAddress,
) -> Result<Decimal, Box<dyn Error>> {
    let account = reader.read_account(wallet).await?;
    usdt_debt(&account)
}

fn usdt_debt(account: &BorrowingAccount) -> Result<Decimal, Box<dyn Error>> {
    account
        .positions
        .iter()
        .flat_map(|position| &position.borrowed)
        .filter(|asset| asset.token.contract == USDT)
        .try_fold(Decimal::ZERO, |total, asset| {
            total
                .checked_add(asset.amount)
                .ok_or_else(|| io::Error::other("indexed USDT debt overflowed").into())
        })
}

fn tslab_supply(account: &BorrowingAccount) -> Result<Decimal, Box<dyn Error>> {
    account
        .positions
        .iter()
        .flat_map(|position| &position.supplied)
        .filter(|asset| asset.token.contract == TSLAB)
        .try_fold(Decimal::ZERO, |total, asset| {
            total
                .checked_add(asset.amount)
                .ok_or_else(|| io::Error::other("indexed TSLAB supply overflowed").into())
        })
}

async fn wait_for_indexed_borrow(
    reader: &impl BorrowingReader,
    wallet: WalletAddress,
    starting_debt: Decimal,
    transaction_hash: B256,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let account = reader.read_account(wallet).await?;
        let borrowed = usdt_debt(&account)?;
        if borrowed > starting_debt
            && account
                .health_factor
                .is_some_and(|health_factor| health_factor > Decimal::ONE)
        {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other(format!(
        "borrow {transaction_hash} succeeded, but Binance did not index it in time"
    ))
    .into())
}

async fn wait_for_indexed_repayment(
    reader: &impl BorrowingReader,
    wallet: WalletAddress,
    transaction_hash: B256,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let account = reader.read_account(wallet).await?;
        if usdt_debt(&account)?.is_zero() && account.health_factor.is_none() {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other(format!(
        "repayment {transaction_hash} succeeded, but Binance did not index it in time"
    ))
    .into())
}

async fn wait_for_indexed_withdrawal(
    reader: &impl BorrowingReader,
    wallet: WalletAddress,
    transaction_hash: B256,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let account = reader.read_account(wallet).await?;
        if tslab_supply(&account)?.is_zero() {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other(format!(
        "withdrawal {transaction_hash} succeeded, but Binance did not index it in time"
    ))
    .into())
}
