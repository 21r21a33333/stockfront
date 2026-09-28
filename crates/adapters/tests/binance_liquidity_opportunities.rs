#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;
mod support;

use std::{collections::HashSet, env, error::Error, io, sync::Arc};

use alloy::{providers::Provider, sol, sol_types::SolCall};
use alloy_primitives::{Address, U256, address, utils::parse_units};
use rust_decimal::Decimal;
use stockfront_core::{
    liquidity::{
        LiquidityAccount, LiquidityClaimAction, LiquidityClaimType, LiquidityPolicy,
        LiquidityService, LiquidityTransactionKind, PrepareLiquidityClaimRequest,
        PrepareLiquidityEntryRequest, PrepareLiquidityWithdrawalRequest, PreparedLiquidityEntry,
    },
    wallet_portfolio::WalletAddress,
};

use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};
use support::live_client;

const NVDAB: Address = address!("02Fca66C1D1aFB4E2A7884261eB00F63598a7436");
const POSITION_ATTEMPTS: usize = 45;
const POSITION_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

sol! {
    #[sol(rpc)]
    interface Erc20Metadata {
        function decimals() external view returns (uint8);
    }

    function approve(address spender, uint256 amount) external returns (bool);
}

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

#[tokio::test]
#[ignore = "creates a live bStock LP position on BSC mainnet and spends tokens and BNB"]
async fn creates_live_bstock_liquidity_position() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let client = Arc::new(live_client()?);
    let reviewed_investments = configured_investments()?;
    let investment_id = reviewed_investments[0].clone();
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(reviewed_investments, 100)?,
    );
    let input_token = env::var("STOCKFRONT_LIQUIDITY_TEST_INPUT_TOKEN")?.parse::<Address>()?;
    let input_amount = env::var("STOCKFRONT_LIQUIDITY_TEST_INPUT_AMOUNT")?.parse::<Decimal>()?;
    let price_range_percent = env::var("STOCKFRONT_LIQUIDITY_TEST_PRICE_RANGE_PERCENT")
        .unwrap_or_else(|_| "5".to_owned())
        .parse::<Decimal>()?;
    let starting_positions = service.positions(context.wallet).await?;
    if starting_positions.positions.iter().any(|position| {
        position
            .investment_ids
            .iter()
            .any(|id| id == &investment_id)
    }) {
        return Ok(());
    }

    let entry = service
        .prepare_entry(&PrepareLiquidityEntryRequest {
            wallet: context.wallet,
            investment_id: investment_id.clone(),
            input: stockfront_core::liquidity::LiquidityAmount {
                token: input_token,
                amount: input_amount,
            },
            price_range_percent,
        })
        .await?;
    ensure_required_balances(&context.provider, context.wallet, &entry).await?;
    let (lp_add, approvals) = validated_entry_transactions(&entry)?;
    let wallet = context.wallet.as_address();
    let starting_nft_balance = token_balance(&context.provider, lp_add.to, wallet).await?;

    let allowances = approvals
        .iter()
        .map(|approval| {
            let call = approveCall::abi_decode(&approval.data)?;
            if call.amount == U256::ZERO {
                return Err(io::Error::other("LP approval amount is zero").into());
            }
            Ok((approval.to, call.spender))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let mut granted_allowances = Vec::with_capacity(approvals.len());
    for (approval, &allowance) in approvals.iter().zip(&allowances) {
        if let Err(error) = send_transaction(
            &context.provider,
            approval.to,
            approval.data.clone(),
            approval.value,
        )
        .await
        {
            let _ = revoke_allowances(&context.provider, &granted_allowances).await;
            return Err(error);
        }
        granted_allowances.push(allowance);
    }

    let lp_result = send_transaction(
        &context.provider,
        lp_add.to,
        lp_add.data.clone(),
        lp_add.value,
    )
    .await;
    let revoke_result = revoke_allowances(&context.provider, &granted_allowances).await;
    lp_result?;
    revoke_result?;
    let final_nft_balance = token_balance(&context.provider, lp_add.to, wallet).await?;
    if final_nft_balance <= starting_nft_balance {
        return Err(io::Error::other("LP add did not mint a position NFT").into());
    }

    wait_for_new_position(
        &service,
        context.wallet,
        &investment_id,
        &starting_positions,
    )
    .await
}

async fn revoke_allowances(
    provider: &impl alloy::providers::Provider,
    allowances: &[(Address, Address)],
) -> Result<(), Box<dyn Error>> {
    for &(token, spender) in allowances {
        send_transaction(
            provider,
            token,
            approveCall {
                spender,
                amount: U256::ZERO,
            }
            .abi_encode()
            .into(),
            U256::ZERO,
        )
        .await?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance credentials and a STOCKFRONT_LIQUIDITY_POSITION_WALLET with a live bStock LP position"]
async fn reads_live_bstock_liquidity_positions() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let wallet = WalletAddress::try_from(
        env::var("STOCKFRONT_LIQUIDITY_POSITION_WALLET")?.parse::<Address>()?,
    )?;

    let account = service.positions(wallet).await?;

    assert!(
        !account.positions.is_empty(),
        "Binance returned no bStock liquidity positions for the configured wallet"
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
            && !position.protocol_id.is_empty()
            && !position.protocol_name.is_empty()
            && position.pool_address != Address::ZERO
            && !position.nft_id.is_empty()
            && position.value_usd >= Decimal::ZERO
            && !position.assets.is_empty()
            && position.assets.iter().all(|asset| {
                asset.token.contract != Address::ZERO && !asset.token.symbol.is_empty()
            })
            && position.lower_tick < position.upper_tick
    }));
    assert!(account.observed_at_ms > 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires Binance credentials and a live bStock LP position with claimable fees"]
async fn prepares_live_bstock_liquidity_fee_claim() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let wallet = configured_position_wallet()?;
    let (request, _) = claimable_fee_request(&service, wallet).await?;

    let action = service.prepare_claim(&request).await?;

    let (claim, preceding) = action
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no LP-fee claim transaction"))?;
    assert_eq!(claim.kind, LiquidityTransactionKind::Claim);
    assert!(
        preceding
            .iter()
            .all(|transaction| transaction.kind == LiquidityTransactionKind::Approve)
    );
    assert!(action.transactions.iter().all(|transaction| {
        transaction.to != Address::ZERO
            && !transaction.data.is_empty()
            && transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO)
            && (transaction.gas_price.is_some_and(|gas| gas > U256::ZERO)
                || transaction
                    .max_fee_per_gas
                    .is_some_and(|gas| gas > U256::ZERO))
    }));
    assert!(
        action
            .preview
            .estimated_network_fee
            .is_some_and(|fee| fee.amount.is_sign_positive())
    );
    assert!(
        action
            .preview
            .balance_changes
            .iter()
            .any(|change| change.amount.is_sign_positive())
    );

    Ok(())
}

#[tokio::test]
#[ignore = "claims live bStock LP fees on BSC mainnet and spends BNB"]
async fn claims_live_bstock_liquidity_fees() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let (request, starting_claimable) = claimable_fee_request(&service, context.wallet).await?;
    if starting_claimable.is_empty() {
        return Err(io::Error::other("the LP position has no claimable fee assets").into());
    }
    let wallet = context.wallet.as_address();
    let mut starting_balances = Vec::with_capacity(starting_claimable.len());
    for &(token, _) in &starting_claimable {
        starting_balances.push((
            token,
            token_balance(&context.provider, token, wallet).await?,
        ));
    }
    let action = service.prepare_claim(&request).await?;
    if action
        .transactions
        .last()
        .map(|transaction| transaction.kind)
        != Some(LiquidityTransactionKind::Claim)
    {
        return Err(io::Error::other("unexpected LP-fee claim transaction sequence").into());
    }

    for transaction in action.transactions {
        send_transaction(
            &context.provider,
            transaction.to,
            transaction.data,
            transaction.value,
        )
        .await?;
    }

    let mut received_fees = false;
    for (token, starting_balance) in starting_balances {
        received_fees |= token_balance(&context.provider, token, wallet).await? > starting_balance;
    }
    if !received_fees {
        return Err(io::Error::other("LP-fee claim did not increase any wallet balance").into());
    }

    wait_for_claimable_fees_to_decrease(
        &service,
        context.wallet,
        &request.nft_id,
        &starting_claimable,
    )
    .await
}

#[tokio::test]
#[ignore = "requires Binance credentials and a live bStock LP position"]
async fn prepares_live_bstock_liquidity_withdrawal() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let wallet = configured_position_wallet()?;
    let (request, _) = liquidity_withdrawal_request(&service, wallet).await?;

    let action = service.prepare_withdrawal(&request).await?;

    let (remove, preceding) = action
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no LP-remove transaction"))?;
    assert_eq!(remove.kind, LiquidityTransactionKind::LpRemove);
    assert!(
        preceding
            .iter()
            .all(|transaction| transaction.kind == LiquidityTransactionKind::Approve)
    );
    assert!(action.transactions.iter().all(|transaction| {
        transaction.to != Address::ZERO
            && !transaction.data.is_empty()
            && transaction.gas_limit.is_some_and(|gas| gas > U256::ZERO)
            && (transaction.gas_price.is_some_and(|gas| gas > U256::ZERO)
                || transaction
                    .max_fee_per_gas
                    .is_some_and(|gas| gas > U256::ZERO))
    }));
    assert!(
        action
            .preview
            .balance_changes
            .iter()
            .any(|change| change.amount.is_sign_positive())
    );
    assert!(
        action
            .preview
            .estimated_network_fee
            .is_some_and(|fee| fee.amount.is_sign_positive())
    );

    Ok(())
}

#[tokio::test]
#[ignore = "fully removes a live bStock LP position on BSC mainnet and spends BNB"]
async fn fully_withdraws_live_bstock_liquidity_position() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let client = Arc::new(live_client()?);
    let service = LiquidityService::new(
        client.clone(),
        client.clone(),
        client,
        LiquidityPolicy::new(Vec::new(), 100)?,
    );
    let (request, assets) = liquidity_withdrawal_request(&service, context.wallet).await?;
    let wallet = context.wallet.as_address();
    let mut starting_balances = Vec::with_capacity(assets.len());
    for token in assets {
        starting_balances.push((
            token,
            token_balance(&context.provider, token, wallet).await?,
        ));
    }
    let action = service.prepare_withdrawal(&request).await?;
    if action
        .transactions
        .last()
        .map(|transaction| transaction.kind)
        != Some(LiquidityTransactionKind::LpRemove)
    {
        return Err(io::Error::other("unexpected LP-remove transaction sequence").into());
    }

    for transaction in action.transactions {
        send_transaction(
            &context.provider,
            transaction.to,
            transaction.data,
            transaction.value,
        )
        .await?;
    }

    let mut received_assets = false;
    for (token, starting_balance) in starting_balances {
        received_assets |=
            token_balance(&context.provider, token, wallet).await? > starting_balance;
    }
    if !received_assets {
        return Err(io::Error::other("LP removal did not increase any wallet balance").into());
    }

    wait_for_position_to_disappear(&service, context.wallet, &request.nft_id).await
}

async fn liquidity_withdrawal_request(
    service: &LiquidityService,
    wallet: WalletAddress,
) -> Result<(PrepareLiquidityWithdrawalRequest, Vec<Address>), Box<dyn Error>> {
    let reviewed = configured_investments()?;
    let position = service
        .positions(wallet)
        .await?
        .positions
        .into_iter()
        .find(|position| {
            position
                .investment_ids
                .iter()
                .any(|id| reviewed.contains(id))
        })
        .ok_or_else(|| io::Error::other("wallet has no configured bStock liquidity position"))?;
    let investment_id = position
        .investment_ids
        .iter()
        .find(|id| reviewed.contains(id))
        .cloned()
        .ok_or_else(|| io::Error::other("position omitted its configured investment ID"))?;
    let assets = position
        .assets
        .iter()
        .filter(|asset| asset.amount.is_sign_positive())
        .map(|asset| asset.token.contract)
        .collect();
    Ok((
        PrepareLiquidityWithdrawalRequest {
            wallet,
            investment_id,
            nft_id: position.nft_id,
            ratio: Decimal::ONE,
        },
        assets,
    ))
}

async fn wait_for_position_to_disappear(
    service: &LiquidityService,
    wallet: WalletAddress,
    nft_id: &str,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        if service
            .positions(wallet)
            .await?
            .positions
            .iter()
            .all(|position| position.nft_id != nft_id)
        {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other("Binance did not remove the exited LP position in time").into())
}

async fn claimable_fee_request(
    service: &LiquidityService,
    wallet: WalletAddress,
) -> Result<(PrepareLiquidityClaimRequest, Vec<(Address, Decimal)>), Box<dyn Error>> {
    let reviewed = configured_investments()?;
    let position = service
        .positions(wallet)
        .await?
        .positions
        .into_iter()
        .find(|position| {
            position
                .claim_actions
                .contains(&LiquidityClaimAction::LpFee)
                && position
                    .investment_ids
                    .iter()
                    .any(|id| reviewed.contains(id))
        })
        .ok_or_else(|| {
            io::Error::other(
                "Binance has not exposed LP_FEE for a configured bStock liquidity position",
            )
        })?;
    let investment_id = position
        .investment_ids
        .iter()
        .find(|id| reviewed.contains(id))
        .cloned()
        .ok_or_else(|| io::Error::other("position omitted its configured investment ID"))?;
    let claimable_tokens = position
        .claimable_assets
        .into_iter()
        .filter(|asset| asset.token.contract != Address::ZERO && asset.amount.is_sign_positive())
        .map(|asset| (asset.token.contract, asset.amount))
        .collect();
    Ok((
        PrepareLiquidityClaimRequest {
            wallet,
            investment_id,
            claim_type: LiquidityClaimType::LpFee,
            nft_id: position.nft_id,
        },
        claimable_tokens,
    ))
}

async fn wait_for_claimable_fees_to_decrease(
    service: &LiquidityService,
    wallet: WalletAddress,
    nft_id: &str,
    starting: &[(Address, Decimal)],
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let account = service.positions(wallet).await?;
        if account
            .positions
            .iter()
            .find(|position| position.nft_id == nft_id)
            .is_some_and(|position| {
                starting.iter().all(|(token, amount)| {
                    position
                        .claimable_assets
                        .iter()
                        .find(|asset| asset.token.contract == *token)
                        .map_or(Decimal::ZERO, |asset| asset.amount)
                        < *amount
                })
            })
        {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other("Binance did not index the reduced LP-fee balance in time").into())
}

fn configured_position_wallet() -> Result<WalletAddress, Box<dyn Error>> {
    let wallet = env::var("STOCKFRONT_LIQUIDITY_POSITION_WALLET")
        .or_else(|_| env::var("STOCKFRONT_TEST_WALLET"))?;
    Ok(WalletAddress::try_from(wallet.parse::<Address>()?)?)
}

async fn ensure_required_balances(
    provider: &impl alloy::providers::Provider,
    wallet: WalletAddress,
    entry: &PreparedLiquidityEntry,
) -> Result<(), Box<dyn Error>> {
    for required in &entry.required_amounts {
        let decimals = token_decimals(provider, required.token).await?;
        let required_raw: U256 = parse_units(&required.amount.to_string(), decimals)?.into();
        let available = token_balance(provider, required.token, wallet.as_address()).await?;
        if available < required_raw {
            return Err(io::Error::other(format!(
                "wallet has insufficient {} balance: requires {required_raw} base units",
                required.token
            ))
            .into());
        }
    }
    Ok(())
}

async fn token_decimals(provider: &impl Provider, token: Address) -> Result<u8, Box<dyn Error>> {
    Ok(Erc20Metadata::new(token, provider)
        .decimals()
        .call()
        .await?)
}

fn validated_entry_transactions(
    entry: &PreparedLiquidityEntry,
) -> Result<
    (
        &stockfront_core::liquidity::PreparedLiquidityTransaction,
        &[stockfront_core::liquidity::PreparedLiquidityTransaction],
    ),
    Box<dyn Error>,
> {
    let (lp_add, approvals) = entry
        .transactions
        .split_last()
        .ok_or_else(|| io::Error::other("Binance returned no LP-add transaction"))?;
    if lp_add.kind != LiquidityTransactionKind::LpAdd
        || approvals
            .iter()
            .any(|transaction| transaction.kind != LiquidityTransactionKind::Approve)
    {
        return Err(io::Error::other("unexpected LP transaction sequence").into());
    }
    Ok((lp_add, approvals))
}

async fn wait_for_new_position(
    service: &LiquidityService,
    wallet: WalletAddress,
    investment_id: &str,
    starting: &LiquidityAccount,
) -> Result<(), Box<dyn Error>> {
    let existing = starting
        .positions
        .iter()
        .map(|position| position.nft_id.as_str())
        .collect::<HashSet<_>>();
    for _ in 0..POSITION_ATTEMPTS {
        let account = service.positions(wallet).await?;
        if account.positions.iter().any(|position| {
            !existing.contains(position.nft_id.as_str())
                && position.investment_ids.iter().any(|id| id == investment_id)
        }) {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other("Binance did not index the new bStock LP position in time").into())
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
