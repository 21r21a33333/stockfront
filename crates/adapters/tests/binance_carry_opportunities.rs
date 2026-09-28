#[path = "support/borrowing.rs"]
mod borrowing_support;
#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;
mod support;

use std::{collections::HashSet, env, error::Error, io, sync::Arc, time::Duration};

use alloy_primitives::{Address, U256, address};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{BorrowingPolicy, BorrowingService},
    carry::{
        CarryOpportunityQuery, CarryPolicy, CarryQualification, CarryService, CarryTransactionKind,
        CarryWithdrawalAmount, EarnActionGateway, EarnDepositCommand, EarnReader,
        EarnRedemptionClaimCommand, PrepareCarryWithdrawalRequest,
    },
    wallet_portfolio::WalletAddress,
};

use borrowing_support::live_venus_adapter;
use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};
use support::live_client;

const TSLAB: alloy_primitives::Address = address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");
const LISUSD: alloy_primitives::Address = address!("0782b6d8c4551B9760e74c0545a9bCD90bdc41E5");

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn discovers_live_usdt_earn_opportunities() -> Result<(), Box<dyn Error>> {
    let opportunities = live_client()?.opportunities(USDT).await?;

    assert!(
        !opportunities.is_empty(),
        "Binance returned no USDT earn opportunities"
    );
    let unique_ids = opportunities
        .iter()
        .map(|opportunity| opportunity.investment_id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(unique_ids.len(), opportunities.len());
    assert!(opportunities.iter().all(|opportunity| {
        !opportunity.investment_id.is_empty()
            && !opportunity.investment_name.is_empty()
            && !opportunity.protocol_id.is_empty()
            && !opportunity.protocol_name.is_empty()
            && opportunity
                .protocol_logo_url
                .as_ref()
                .is_none_or(|url| !url.is_empty())
            && opportunity.asset.contract == USDT
            && !opportunity.asset.symbol.is_empty()
            && !opportunity.base_rate_display.is_empty()
            && opportunity
                .reported_tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
            && opportunity.observed_at_ms > 0
    }));

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials and reviewed carry configuration"]
async fn configured_live_carry_investment_is_eligible() -> Result<(), Box<dyn Error>> {
    let carry = live_carry_service().await?;
    let investment_id = configured_investment_id()?;
    let opportunities = carry
        .opportunities(CarryOpportunityQuery {
            collateral_token: TSLAB,
            debt_token: USDT,
        })
        .await?;

    let opportunity = opportunities
        .iter()
        .find(|opportunity| opportunity.investment_id == investment_id)
        .ok_or_else(|| {
            io::Error::other(format!(
                "configured carry investment {investment_id} is not available for TSLAB/USDT"
            ))
        })?;
    assert_eq!(opportunity.qualification, CarryQualification::Eligible);
    assert!(opportunity.reasons.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and a reviewed carry investment"]
async fn prepares_live_carry_deposit() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let investment_id = configured_investment_id()?;
    let opportunity = client
        .opportunity(&investment_id)
        .await?
        .ok_or_else(|| io::Error::other("reviewed carry investment was not found"))?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;
    let amount = env::var("STOCKFRONT_CARRY_TEST_AMOUNT")
        .unwrap_or_else(|_| "1".to_owned())
        .parse::<Decimal>()?;

    let plan = client
        .prepare_deposit(&EarnDepositCommand {
            wallet,
            investment_id,
            asset: opportunity.asset.contract,
            amount,
        })
        .await?;

    let (deposit, approvals) = plan
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no deposit transaction"))?;
    assert_eq!(deposit.kind, CarryTransactionKind::Deposit);
    assert!(
        approvals
            .iter()
            .all(|transaction| transaction.kind == CarryTransactionKind::Approve)
    );
    for transaction in &plan.transactions {
        assert_ne!(transaction.to, Address::ZERO);
        assert!(!transaction.data.is_empty());
        assert_eq!(transaction.value, U256::ZERO);
        assert!(transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO));
        assert!(
            transaction
                .max_priority_fee_per_gas
                .is_some_and(|fee| fee > U256::ZERO)
        );
        assert!(
            transaction
                .max_fee_per_gas
                .is_some_and(|fee| fee > U256::ZERO)
        );
    }
    assert_eq!(plan.preview.interact_with, Some(deposit.to));
    let fee = plan
        .preview
        .estimated_network_fee
        .ok_or_else(|| io::Error::other("network fee missing from deposit preview"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert!(fee.value_usd.is_some_and(|value| value.is_sign_positive()));
    assert!(plan.preview.balance_changes.iter().any(|change| {
        change.token.contract == opportunity.asset.contract && change.amount == -amount
    }));
    assert!(plan.preview.warnings.iter().all(|warning| {
        !warning.code.is_empty() && !warning.level.is_empty() && !warning.message.is_empty()
    }));

    Ok(())
}

#[tokio::test]
#[ignore = "deposits 1 borrowed USDT into Lista on BSC mainnet and spends BNB for gas"]
async fn deposits_live_borrowed_usdt_into_lista() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let client = live_client()?;
    let investment_id = configured_investment_id()?;
    let amount = Decimal::ONE;
    let amount_raw = U256::from(1_000_000_000_000_000_000_u64);
    let wallet = context.wallet.as_address();
    let starting_usdt = token_balance(&context.provider, USDT, wallet).await?;
    if starting_usdt < amount_raw {
        return Err(io::Error::other("wallet needs at least 1 USDT for the Lista deposit").into());
    }

    let plan = client
        .prepare_deposit(&EarnDepositCommand {
            wallet: context.wallet,
            investment_id: investment_id.clone(),
            asset: USDT,
            amount,
        })
        .await?;
    let receipt_tokens = plan
        .preview
        .balance_changes
        .iter()
        .filter(|change| change.amount.is_sign_positive())
        .map(|change| change.token.contract)
        .collect::<HashSet<_>>();
    if receipt_tokens.is_empty() {
        return Err(io::Error::other("deposit preview contains no receipt asset").into());
    }
    let mut starting_receipt_balances = Vec::with_capacity(receipt_tokens.len());
    for token in receipt_tokens {
        starting_receipt_balances.push((
            token,
            token_balance(&context.provider, token, wallet).await?,
        ));
    }

    for transaction in &plan.transactions {
        send_transaction(
            &context.provider,
            transaction.to,
            transaction.data.clone(),
            transaction.value,
        )
        .await?;
    }

    assert_eq!(
        token_balance(&context.provider, USDT, wallet).await?,
        starting_usdt - amount_raw
    );
    let mut received_position_token = false;
    for (token, starting_balance) in starting_receipt_balances {
        if token_balance(&context.provider, token, wallet).await? > starting_balance {
            received_position_token = true;
            break;
        }
    }
    assert!(
        received_position_token,
        "no Lista receipt balance increased"
    );
    wait_for_carry_position(&client, context.wallet, &investment_id).await
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and a carry position"]
async fn prepares_live_full_carry_withdrawal() -> Result<(), Box<dyn Error>> {
    let carry = live_carry_service().await?;
    let investment_id = configured_investment_id()?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;

    let plan = carry
        .prepare_withdrawal(&PrepareCarryWithdrawalRequest {
            wallet,
            investment_id,
            amount: CarryWithdrawalAmount::Full,
        })
        .await?;
    assert_valid_withdrawal_plan(&plan);

    Ok(())
}

#[tokio::test]
#[ignore = "fully redeems the live Lista position on BSC mainnet and spends BNB for gas"]
async fn fully_redeems_live_lista_position() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let client = live_client()?;
    let carry = live_carry_service().await?;
    let investment_id = configured_investment_id()?;
    let account_before = client.positions(context.wallet).await?;
    let position_before = carry_asset_amount(&account_before, &investment_id, LISUSD);
    if position_before <= Decimal::ZERO {
        return Err(io::Error::other("wallet has no live Lista position to redeem").into());
    }
    let wallet = context.wallet.as_address();
    let balance_before = token_balance(&context.provider, LISUSD, wallet).await?;

    let plan = carry
        .prepare_withdrawal(&PrepareCarryWithdrawalRequest {
            wallet: context.wallet,
            investment_id: investment_id.clone(),
            amount: CarryWithdrawalAmount::Full,
        })
        .await?;
    assert_valid_withdrawal_plan(&plan);
    assert!(
        plan.preview
            .balance_changes
            .iter()
            .any(|change| { change.token.contract == LISUSD && change.amount == position_before })
    );

    for transaction in &plan.transactions {
        send_transaction(
            &context.provider,
            transaction.to,
            transaction.data.clone(),
            transaction.value,
        )
        .await?;
    }

    assert!(
        token_balance(&context.provider, LISUSD, wallet).await? > balance_before,
        "Lista redemption did not increase the wallet's lisUSD balance"
    );
    wait_for_carry_position_reduction(&client, context.wallet, &investment_id, position_before)
        .await
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and a claimable redemption"]
async fn prepares_live_carry_redemption_claim() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;
    let investment_id = env::var("STOCKFRONT_CARRY_INVESTMENT_ID")?;
    let redemption_id = env::var("STOCKFRONT_CARRY_REDEMPTION_ID")?;

    let plan = client
        .prepare_redemption_claim(&EarnRedemptionClaimCommand {
            wallet,
            investment_id,
            redemption_id,
        })
        .await?;

    let (claim, preceding) = plan
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no claim transaction"))?;
    assert_eq!(claim.kind, CarryTransactionKind::Claim);
    assert!(
        preceding
            .iter()
            .all(|transaction| transaction.kind == CarryTransactionKind::Approve)
    );
    assert!(plan.transactions.iter().all(|transaction| {
        transaction.to != Address::ZERO
            && !transaction.data.is_empty()
            && transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO)
    }));
    assert!(
        plan.preview
            .estimated_network_fee
            .is_some_and(|fee| fee.amount.is_sign_positive())
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials and a STOCKFRONT_TEST_WALLET with earn positions"]
async fn reads_live_carry_positions() -> Result<(), Box<dyn Error>> {
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;
    let account = live_client()?.positions(wallet).await?;

    assert_eq!(account.chain_id, 56);
    assert!(account.observed_at_ms > 0);
    assert!(
        !account.positions.is_empty(),
        "the test wallet has no reviewed carry position"
    );
    assert_eq!(
        account.total_value_usd,
        account
            .positions
            .iter()
            .map(|position| position.value_usd)
            .sum()
    );
    assert!(account.positions.iter().all(|position| {
        !position.position_id.is_empty()
            && !position.investment_ids.is_empty()
            && position
                .investment_ids
                .iter()
                .all(|investment_id| !investment_id.is_empty())
            && !position.protocol_id.is_empty()
            && !position.protocol_name.is_empty()
            && position.value_usd >= Decimal::ZERO
            && !position.assets.is_empty()
            && position.assets.iter().all(|asset| {
                asset.token.contract != Address::ZERO
                    && !asset.token.symbol.is_empty()
                    && asset.amount >= Decimal::ZERO
                    && asset.value_usd >= Decimal::ZERO
            })
    }));

    Ok(())
}

fn configured_investment_id() -> Result<String, Box<dyn Error>> {
    env::var("STOCKFRONT_CARRY_INVESTMENT_IDS")?
        .split(',')
        .map(str::trim)
        .find(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| io::Error::other("no reviewed carry investment is configured").into())
}

fn assert_valid_withdrawal_plan(plan: &stockfront_core::carry::CarryActionPlan) {
    let (redeem, preceding) = plan
        .transactions
        .split_last()
        .unwrap_or_else(|| panic!("Binance returned no redemption transaction"));
    assert_eq!(redeem.kind, CarryTransactionKind::Redeem);
    assert!(
        preceding
            .iter()
            .all(|transaction| transaction.kind == CarryTransactionKind::Approve)
    );
    assert!(plan.transactions.iter().all(|transaction| {
        transaction.to != Address::ZERO
            && !transaction.data.is_empty()
            && transaction.value == U256::ZERO
            && transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO)
            && transaction.gas_price.is_some_and(|gas| gas > U256::ZERO)
            && transaction
                .max_priority_fee_per_gas
                .is_some_and(|gas| gas > U256::ZERO)
            && transaction
                .max_fee_per_gas
                .is_some_and(|gas| gas > U256::ZERO)
    }));
    assert_eq!(plan.preview.interact_with, Some(redeem.to));
    assert!(!plan.preview.balance_changes.is_empty());
    assert!(plan.preview.balance_changes.iter().all(|change| {
        change.token.contract != Address::ZERO
            && !change.token.symbol.is_empty()
            && change.amount != Decimal::ZERO
            && change
                .value_usd
                .is_none_or(|value| value.is_sign_positive())
    }));
    let fee = plan
        .preview
        .estimated_network_fee
        .as_ref()
        .unwrap_or_else(|| panic!("network fee missing from withdrawal preview"));
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert!(fee.value_usd.is_none_or(|value| value.is_sign_positive()));
    assert!(plan.preview.warnings.iter().all(|warning| {
        !warning.code.is_empty() && !warning.level.is_empty() && !warning.message.is_empty()
    }));
}

async fn live_carry_service() -> Result<CarryService, Box<dyn Error>> {
    let reviewed_investment_ids = env::var("STOCKFRONT_CARRY_INVESTMENT_IDS")?
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let minimum_base_spread_bps = env::var("STOCKFRONT_MIN_CARRY_SPREAD_BPS")
        .unwrap_or_else(|_| "100".to_owned())
        .parse()?;
    let binance = Arc::new(live_client()?);
    let venus = Arc::new(live_venus_adapter(true).await?);
    let borrowing = Arc::new(BorrowingService::new(
        binance.clone(),
        venus.clone(),
        venus,
        BorrowingPolicy::new(vec![USDT])?,
    ));

    Ok(CarryService::new(
        borrowing,
        binance.clone(),
        binance,
        CarryPolicy::new(reviewed_investment_ids, minimum_base_spread_bps),
    ))
}

async fn wait_for_carry_position(
    client: &stockfront_adapters::binance_web3::BinanceWeb3Client,
    wallet: WalletAddress,
    investment_id: &str,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..30 {
        let account = client.positions(wallet).await?;
        if account
            .positions
            .iter()
            .any(|position| position.investment_ids.iter().any(|id| id == investment_id))
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(io::Error::other("Binance did not index the Lista position within 60 seconds").into())
}

fn carry_asset_amount(
    account: &stockfront_core::carry::CarryAccount,
    investment_id: &str,
    token: Address,
) -> Decimal {
    account
        .positions
        .iter()
        .filter(|position| position.investment_ids.iter().any(|id| id == investment_id))
        .flat_map(|position| &position.assets)
        .filter(|asset| asset.token.contract == token)
        .map(|asset| asset.amount)
        .sum()
}

async fn wait_for_carry_position_reduction(
    client: &stockfront_adapters::binance_web3::BinanceWeb3Client,
    wallet: WalletAddress,
    investment_id: &str,
    amount_before: Decimal,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..30 {
        let account = client.positions(wallet).await?;
        if carry_asset_amount(&account, investment_id, LISUSD) < amount_before {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(io::Error::other("Binance did not index the Lista redemption within 60 seconds").into())
}
