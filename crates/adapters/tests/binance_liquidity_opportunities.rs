mod support;

use std::{collections::HashSet, env, error::Error, io, sync::Arc};

use alloy_primitives::{Address, U256, address};
use rust_decimal::Decimal;
use stockfront_core::{
    liquidity::{
        LiquidityPolicy, LiquidityService, LiquidityTransactionKind, PrepareLiquidityEntryRequest,
    },
    wallet_portfolio::WalletAddress,
};

use support::live_client;

const NVDAB: Address = address!("02Fca66C1D1aFB4E2A7884261eB00F63598a7436");

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn discovers_live_nvidia_bstock_liquidity_opportunities() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let opportunities = service.opportunities(NVDAB).await?;

    assert!(
        !opportunities.is_empty(),
        "Binance returned no NVIDIA bStock liquidity opportunities"
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
            && opportunity
                .pool_address
                .is_none_or(|address| address != Address::ZERO)
            && opportunity.assets.len() == 2
            && opportunity
                .assets
                .iter()
                .all(|asset| asset.contract != Address::ZERO && !asset.symbol.is_empty())
            && opportunity
                .assets
                .iter()
                .any(|asset| asset.contract == NVDAB)
            && !opportunity.reported_rate_display.is_empty()
            && opportunity
                .reported_tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
            && opportunity.fee_rate.is_none_or(|fee| fee >= Decimal::ZERO)
            && opportunity.observed_at_ms > 0
    }));

    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance credentials, a funded STOCKFRONT_TEST_WALLET, and reviewed liquidity configuration"]
async fn prepares_live_bstock_liquidity_entry() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let reviewed_investments = configured_investments()?;
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(reviewed_investments.clone(), 100)?,
    );
    let input_token = env::var("STOCKFRONT_LIQUIDITY_TEST_INPUT_TOKEN")?.parse::<Address>()?;
    let input_amount = env::var("STOCKFRONT_LIQUIDITY_TEST_INPUT_AMOUNT")?.parse::<Decimal>()?;
    let price_range_percent = env::var("STOCKFRONT_LIQUIDITY_TEST_PRICE_RANGE_PERCENT")
        .unwrap_or_else(|_| "5".to_owned())
        .parse::<Decimal>()?;
    let wallet = WalletAddress::try_from(env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?)?;
    let entry = service
        .prepare_entry(&PrepareLiquidityEntryRequest {
            wallet,
            investment_id: reviewed_investments[0].clone(),
            input: stockfront_core::liquidity::LiquidityAmount {
                token: input_token,
                amount: input_amount,
            },
            price_range_percent,
        })
        .await?;

    assert_eq!(entry.required_amounts.len(), 2);
    assert!(
        entry
            .required_amounts
            .iter()
            .all(|amount| { amount.token != Address::ZERO && amount.amount.is_sign_positive() })
    );
    assert!(
        entry
            .required_amounts
            .iter()
            .any(|amount| { amount.token == input_token && amount.amount == input_amount })
    );
    assert_eq!(
        entry
            .required_amounts
            .iter()
            .map(|amount| amount.token)
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    let final_transaction = entry
        .transactions
        .last()
        .ok_or_else(|| io::Error::other("Binance returned no LP-add transaction"))?;
    assert_eq!(final_transaction.kind, LiquidityTransactionKind::LpAdd);
    assert!(
        entry.transactions[..entry.transactions.len() - 1]
            .iter()
            .all(|transaction| transaction.kind == LiquidityTransactionKind::Approve)
    );
    assert!(entry.transactions.iter().all(|transaction| {
        transaction.to != Address::ZERO
            && !transaction.data.is_empty()
            && transaction.value == U256::ZERO
            && transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO)
            && (transaction.gas_price.is_some_and(|gas| gas > U256::ZERO)
                || transaction
                    .max_fee_per_gas
                    .is_some_and(|gas| gas > U256::ZERO))
    }));
    assert!(!entry.preview.balance_changes.is_empty());
    assert!(entry.preview.balance_changes.iter().all(|change| {
        change.token.contract != Address::ZERO
            && !change.token.symbol.is_empty()
            && change.amount != Decimal::ZERO
            && change
                .value_usd
                .is_none_or(|value| value.is_sign_positive())
    }));
    let fee = entry
        .preview
        .estimated_network_fee
        .ok_or_else(|| io::Error::other("LP-add preview omitted its network fee"))?;
    assert!(fee.amount.is_sign_positive());
    assert_eq!(fee.token_symbol, "BNB");
    assert!(fee.value_usd.is_none_or(|value| value.is_sign_positive()));
    assert!(
        entry
            .preview
            .interact_with
            .is_some_and(|address| address != Address::ZERO)
    );
    assert!(entry.preview.warnings.iter().all(|warning| {
        !warning.code.is_empty() && !warning.level.is_empty() && !warning.message.is_empty()
    }));

    Ok(())
}

fn configured_investments() -> Result<Vec<String>, Box<dyn Error>> {
    let investment_ids = env::var("STOCKFRONT_LIQUIDITY_INVESTMENT_IDS")?
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if investment_ids.is_empty() {
        return Err(io::Error::other("no reviewed liquidity investment is configured").into());
    }
    Ok(investment_ids)
}
