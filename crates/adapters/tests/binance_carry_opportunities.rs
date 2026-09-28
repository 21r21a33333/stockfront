mod support;

use std::{collections::HashSet, env, error::Error, io};

use alloy_primitives::{Address, U256, address};
use rust_decimal::Decimal;
use stockfront_core::{
    carry::{
        CarryTransactionKind, CarryWithdrawalAmount, EarnActionGateway, EarnDepositCommand,
        EarnReader, EarnWithdrawalCommand,
    },
    wallet_portfolio::WalletAddress,
};

use support::live_client;

const USDT: alloy_primitives::Address = address!("55d398326f99059fF775485246999027B3197955");

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
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and a reviewed carry investment"]
async fn prepares_live_carry_deposit() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let investment_id = env::var("STOCKFRONT_CARRY_INVESTMENT_IDS")?
        .split(',')
        .map(str::trim)
        .find(|id| !id.is_empty())
        .ok_or_else(|| io::Error::other("no reviewed carry investment is configured"))?
        .to_owned();
    let opportunity = client
        .opportunity(&investment_id)
        .await?
        .ok_or_else(|| io::Error::other("reviewed carry investment was not found"))?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;
    let amount = env::var("STOCKFRONT_CARRY_TEST_AMOUNT")
        .unwrap_or_else(|_| "0.01".to_owned())
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
#[ignore = "requires live Binance Web3 credentials, STOCKFRONT_TEST_WALLET, and a carry position"]
async fn prepares_live_full_carry_withdrawal() -> Result<(), Box<dyn Error>> {
    let client = live_client()?;
    let investment_id = env::var("STOCKFRONT_CARRY_INVESTMENT_ID")?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;

    let plan = client
        .prepare_withdrawal(&EarnWithdrawalCommand {
            wallet,
            investment_id,
            amount: CarryWithdrawalAmount::Full,
        })
        .await?;

    let (redeem, preceding) = plan
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no redemption transaction"))?;
    assert_eq!(redeem.kind, CarryTransactionKind::Redeem);
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
