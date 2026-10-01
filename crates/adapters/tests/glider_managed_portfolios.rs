#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;

use std::{collections::HashSet, error::Error, io, time::Duration};

use alloy::{
    dyn_abi::TypedData,
    primitives::Address,
    providers::DynProvider,
    signers::{Signer, local::PrivateKeySigner},
    sol,
    sol_types::SolCall,
};
use alloy_primitives::U256;
use rust_decimal::Decimal;
use stockfront_adapters::glider::GliderAdapter;
use stockfront_core::{
    baskets::BasketKind,
    bstock_market::BNB_CHAIN_ID,
    managed_portfolios::{
        EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery,
        ListManagedPortfoliosQuery, ManagedPortfolioCollection, ManagedPortfolioGateway,
        ManagedPortfolioOperationState, ManagedPortfolioProviderError, ManagedPortfolioSort,
        ManagedPortfolioTransactionKind, ManagedPortfolioWithdrawalAsset,
        PrepareManagedPortfolioDepositRequest, PrepareManagedPortfolioEnrollmentRequest,
        PrepareManagedPortfolioWithdrawalRequest, PreparedManagedPortfolioDeposit,
        PreparedManagedPortfolioTransaction, PreparedManagedPortfolioWithdrawal,
        SubmitManagedPortfolioWithdrawalRequest,
    },
    wallet_portfolio::WalletAddress,
};
use zeroize::Zeroizing;

use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};

const ENROLLMENT_CONFIRMATION: &str = "I_UNDERSTAND_THIS_CREATES_A_MANAGED_PORTFOLIO";
const BSC_USDT: Address = alloy::primitives::address!("55d398326f99059ff775485246999027b3197955");
const ONE_USDT_RAW: u128 = 1_000_000_000_000_000_000;
const POSITION_ATTEMPTS: usize = 30;
const POSITION_POLL_INTERVAL: Duration = Duration::from_secs(2);
const WITHDRAWAL_CONFIRMATION: &str = "I_UNDERSTAND_THIS_WITHDRAWS_REAL_FUNDS";

sol! {
    function transfer(address recipient, uint256 amount) external returns (bool);
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn discovers_live_bnb_managed_portfolio_templates() -> Result<(), Box<dyn Error>> {
    let reader = live_gateway()?;
    let query = ListManagedPortfolioTemplatesQuery::new(
        Some(ManagedPortfolioCollection::Curated),
        None,
        Some(50),
        None,
    )?;

    let page = reader.templates(&query).await?;

    assert!(
        !page.items.is_empty(),
        "Glider returned no curated BNB templates"
    );
    for template in page.items {
        assert!(!template.id.is_empty());
        assert_eq!(template.kind, BasketKind::ManagedPortfolio);
        assert_eq!(template.provider_id, "glider");
        assert_eq!(template.provider_name, "Glider");
        assert!(!template.name.is_empty());
        assert_eq!(template.name, template.name.trim());
        assert!(
            template
                .description
                .as_ref()
                .is_none_or(|description| !description.is_empty())
        );
        assert!(
            template
                .advertised_max_apy_percent
                .is_none_or(|apy| apy >= Decimal::ZERO)
        );
        assert!(!template.allocation.is_empty());
        assert_eq!(
            template
                .allocation
                .iter()
                .map(|asset| asset.weight_percent)
                .sum::<Decimal>(),
            Decimal::ONE_HUNDRED
        );
        for asset in template.allocation {
            assert_eq!(asset.chain_id, BNB_CHAIN_ID);
            assert_ne!(asset.contract, alloy_primitives::Address::ZERO);
            assert!(asset.weight_percent > Decimal::ZERO);
        }
        assert!(
            template
                .metrics
                .tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
        );
        for performance in template.metrics.performance {
            assert!(!performance.window.is_empty());
            assert!(!performance.since.is_empty());
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn follows_live_managed_portfolio_template_cursor() -> Result<(), Box<dyn Error>> {
    let reader = live_gateway()?;
    let first = reader
        .templates(&ListManagedPortfolioTemplatesQuery::new(
            Some(ManagedPortfolioCollection::TopPerforming),
            Some(ManagedPortfolioSort::Tvl),
            Some(5),
            None,
        )?)
        .await?;
    let cursor = first
        .next_cursor
        .ok_or("Glider did not return a cursor for a partial ranked page")?;
    let second = reader
        .templates(&ListManagedPortfolioTemplatesQuery::new(
            Some(ManagedPortfolioCollection::TopPerforming),
            Some(ManagedPortfolioSort::Tvl),
            Some(5),
            Some(cursor),
        )?)
        .await?;

    assert!(!first.items.is_empty() || !second.items.is_empty());
    let first_ids = first
        .items
        .iter()
        .map(|template| template.id.as_str())
        .collect::<HashSet<_>>();
    assert!(
        second
            .items
            .iter()
            .all(|template| !first_ids.contains(template.id.as_str()))
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn prepares_live_bnb_managed_portfolio_enrollment() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let prepared = gateway
        .prepare_enrollment(&PrepareManagedPortfolioEnrollmentRequest {
            wallet: test_wallet()?,
            template_id: std::env::var("STOCKFRONT_GLIDER_TEMPLATE_ID")?,
        })
        .await?;

    assert!(!prepared.authorization.flow_id.is_empty());
    assert!(!prepared.authorization.account_index.is_empty());
    assert!(!prepared.authorization.agent_account_id.is_empty());
    assert_ne!(
        prepared.personal_sign_message,
        alloy::primitives::B256::ZERO
    );
    Ok(())
}

#[tokio::test]
#[ignore = "creates a live Glider managed portfolio"]
async fn enrolls_live_bnb_managed_portfolio() -> Result<(), Box<dyn Error>> {
    if std::env::var("STOCKFRONT_GLIDER_ENROLL_CONFIRM")
        .ok()
        .as_deref()
        != Some(ENROLLMENT_CONFIRMATION)
    {
        return Err(io::Error::other(format!(
            "set STOCKFRONT_GLIDER_ENROLL_CONFIRM={ENROLLMENT_CONFIRMATION} to create a portfolio"
        ))
        .into());
    }

    let gateway = live_gateway()?;
    let private_key = Zeroizing::new(std::env::var("STOCKFRONT_E2E_PRIVATE_KEY")?);
    let signer = private_key.parse::<PrivateKeySigner>()?;
    drop(private_key);
    let wallet = WalletAddress::try_from(signer.address())?;
    let template_id = std::env::var("STOCKFRONT_GLIDER_TEMPLATE_ID")?;
    let prepared = gateway
        .prepare_enrollment(&PrepareManagedPortfolioEnrollmentRequest {
            wallet,
            template_id: template_id.clone(),
        })
        .await?;
    let signature = signer
        .sign_message(prepared.personal_sign_message.as_slice())
        .await?
        .to_string();
    let enrollment = gateway
        .enroll(&EnrollManagedPortfolioRequest {
            wallet,
            template_id: template_id.clone(),
            authorization: prepared.authorization,
            signature,
            name: std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_NAME").ok(),
        })
        .await?;

    assert!(!enrollment.portfolio_id.is_empty());
    assert_eq!(enrollment.template_id, template_id);
    assert_eq!(enrollment.smart_account.chain_id, BNB_CHAIN_ID);
    assert_ne!(enrollment.smart_account.address, Address::ZERO);
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn lists_live_bnb_managed_portfolios_for_wallet() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let page = gateway
        .portfolios(
            test_wallet()?,
            &ListManagedPortfoliosQuery::new(Some(50), None)?,
        )
        .await?;

    assert!(!page.items.is_empty(), "wallet has no Glider portfolios");
    for portfolio in page.items {
        assert!(!portfolio.portfolio_id.is_empty());
        assert!(!portfolio.template_id.is_empty());
        assert!(!portfolio.template_name.is_empty());
        assert_eq!(portfolio.smart_account.chain_id, BNB_CHAIN_ID);
        assert_ne!(portfolio.smart_account.address, Address::ZERO);
        assert!(
            portfolio
                .name
                .as_ref()
                .is_none_or(|name| !name.trim().is_empty())
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn reads_live_bnb_managed_portfolio_positions() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let positions = gateway
        .positions(&std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?)
        .await?;

    assert!(!positions.portfolio_id.is_empty());
    assert_eq!(positions.smart_account.chain_id, BNB_CHAIN_ID);
    assert_ne!(positions.smart_account.address, Address::ZERO);
    assert!(positions.total_value_usd >= Decimal::ZERO);
    assert!(positions.observed_at_ms > 0);
    for asset in positions.assets {
        assert_ne!(asset.contract, Address::ZERO);
        assert!(!asset.symbol.is_empty());
        assert!(asset.amount >= Decimal::ZERO);
        assert!(asset.price_usd >= Decimal::ZERO);
        assert!(asset.value_usd >= Decimal::ZERO);
    }
    for warning in positions.warnings {
        assert!(!warning.message.trim().is_empty());
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials and a BNB managed portfolio"]
async fn prepares_live_bnb_managed_portfolio_deposit() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let token = std::env::var("STOCKFRONT_GLIDER_DEPOSIT_TOKEN")?.parse::<Address>()?;
    let raw_amount = std::env::var("STOCKFRONT_GLIDER_DEPOSIT_RAW_AMOUNT")?.parse::<U256>()?;
    let prepared = gateway
        .prepare_deposit(
            &std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?,
            &PrepareManagedPortfolioDepositRequest { token, raw_amount },
        )
        .await?;

    validated_deposit_transaction(&prepared, token, raw_amount)?;
    assert!(prepared.preview.balance_changes.is_empty());
    assert_eq!(prepared.preview.estimated_network_fee, None);
    assert!(prepared.preview.warnings.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore = "transfers 1 USDT to a live Glider portfolio on BSC mainnet and spends BNB for gas"]
async fn funds_live_bnb_managed_portfolio_with_one_usdt() -> Result<(), Box<dyn Error>> {
    let context = live_evm_context().await?;
    let gateway = live_gateway()?;
    let portfolio_id = std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?;
    let raw_amount = U256::from(ONE_USDT_RAW);
    let wallet = context.wallet.as_address();
    let starting_wallet_balance = token_balance(&context.provider, BSC_USDT, wallet).await?;
    if starting_wallet_balance < raw_amount {
        return Err(io::Error::other("wallet needs at least 1 USDT").into());
    }
    let starting_positions = gateway.positions(&portfolio_id).await?;
    let prepared = gateway
        .prepare_deposit(
            &portfolio_id,
            &PrepareManagedPortfolioDepositRequest {
                token: BSC_USDT,
                raw_amount,
            },
        )
        .await?;
    let transaction = validated_deposit_transaction(&prepared, BSC_USDT, raw_amount)?;

    send_transaction(
        &context.provider,
        transaction.to,
        transaction.data.clone(),
        transaction.value,
    )
    .await?;

    let final_wallet_balance = token_balance(&context.provider, BSC_USDT, wallet).await?;
    assert_eq!(
        final_wallet_balance,
        starting_wallet_balance
            .checked_sub(raw_amount)
            .ok_or_else(|| io::Error::other("USDT balance subtraction underflowed"))?
    );

    wait_for_deposit_reconciliation(&gateway, &portfolio_id, starting_positions.total_value_usd)
        .await
}

#[tokio::test]
#[ignore = "requires live Glider API credentials and a funded BNB managed portfolio"]
async fn prepares_live_in_kind_managed_portfolio_withdrawal() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let portfolio_id = std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?;
    let positions = gateway.positions(&portfolio_id).await?;
    let asset = positions
        .assets
        .first()
        .ok_or_else(|| io::Error::other("portfolio has no BNB assets to withdraw"))?;
    let recipient = test_wallet()?;
    let prepared = gateway
        .prepare_withdrawal(
            &portfolio_id,
            &PrepareManagedPortfolioWithdrawalRequest::InKind {
                recipient,
                assets: vec![ManagedPortfolioWithdrawalAsset {
                    token: asset.contract,
                    raw_amount: asset.raw_amount,
                }],
            },
        )
        .await?;

    validate_prepared_withdrawal(&prepared, &portfolio_id, recipient, false)?;
    assert_eq!(prepared.typed_data.message.assets.len(), 1);
    assert_eq!(
        prepared.typed_data.message.assets[0].amount_raw,
        asset.raw_amount.to_string()
    );
    assert_eq!(prepared.typed_data.message.settlement_asset_id, None);
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials and a funded BNB managed portfolio"]
async fn prepares_live_full_usdt_managed_portfolio_withdrawal() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let portfolio_id = std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?;
    let recipient = test_wallet()?;
    let positions = gateway.positions(&portfolio_id).await?;
    if positions.total_value_usd <= Decimal::ONE {
        return Err(io::Error::other(
            "portfolio must hold more than Glider's $1 liquidation threshold",
        )
        .into());
    }
    let prepared = gateway
        .prepare_withdrawal(
            &portfolio_id,
            &PrepareManagedPortfolioWithdrawalRequest::FullToUsdt { recipient },
        )
        .await?;

    validate_prepared_withdrawal(&prepared, &portfolio_id, recipient, true)?;
    assert!(!prepared.typed_data.message.assets.is_empty());
    assert_eq!(
        prepared.typed_data.message.settlement_asset_id.as_deref(),
        Some("eip155:56/erc20:0x55d398326f99059ff775485246999027b3197955")
    );
    Ok(())
}

#[tokio::test]
#[ignore = "withdraws one live BNB managed-portfolio asset to the owner wallet"]
async fn withdraws_live_managed_portfolio_asset_in_kind() -> Result<(), Box<dyn Error>> {
    require_withdrawal_confirmation()?;
    let context = live_evm_context().await?;
    let signer = live_signer()?;
    let gateway = live_gateway()?;
    let portfolio_id = std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?;
    let positions = gateway.positions(&portfolio_id).await?;
    let asset = positions
        .assets
        .first()
        .ok_or_else(|| io::Error::other("portfolio has no BNB assets to withdraw"))?;
    let starting_balance = token_balance(
        &context.provider,
        asset.contract,
        context.wallet.as_address(),
    )
    .await?;
    let prepared = gateway
        .prepare_withdrawal(
            &portfolio_id,
            &PrepareManagedPortfolioWithdrawalRequest::InKind {
                recipient: context.wallet,
                assets: vec![ManagedPortfolioWithdrawalAsset {
                    token: asset.contract,
                    raw_amount: asset.raw_amount,
                }],
            },
        )
        .await?;
    let submitted = sign_and_submit(&gateway, &signer, &portfolio_id, prepared).await?;
    wait_for_withdrawn_balance(
        &gateway,
        &portfolio_id,
        &submitted.operation_id,
        &context.provider,
        asset.contract,
        context.wallet.as_address(),
        starting_balance + asset.raw_amount,
    )
    .await
}

#[tokio::test]
#[ignore = "liquidates a live BNB managed portfolio and withdraws USDT to the owner wallet"]
async fn fully_liquidates_live_managed_portfolio_to_usdt() -> Result<(), Box<dyn Error>> {
    require_withdrawal_confirmation()?;
    let context = live_evm_context().await?;
    let signer = live_signer()?;
    let gateway = live_gateway()?;
    let portfolio_id = std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_ID")?;
    let starting_balance =
        token_balance(&context.provider, BSC_USDT, context.wallet.as_address()).await?;
    let prepared = gateway
        .prepare_withdrawal(
            &portfolio_id,
            &PrepareManagedPortfolioWithdrawalRequest::FullToUsdt {
                recipient: context.wallet,
            },
        )
        .await?;
    let submitted = sign_and_submit(&gateway, &signer, &portfolio_id, prepared).await?;
    wait_for_full_liquidation(
        &gateway,
        &portfolio_id,
        &submitted.operation_id,
        &context.provider,
        context.wallet.as_address(),
        starting_balance,
    )
    .await
}

fn validated_deposit_transaction(
    prepared: &PreparedManagedPortfolioDeposit,
    token: Address,
    raw_amount: U256,
) -> Result<&PreparedManagedPortfolioTransaction, Box<dyn Error>> {
    let [transaction] = prepared.transactions.as_slice() else {
        return Err(io::Error::other("deposit must contain exactly one transaction").into());
    };
    let account = prepared
        .preview
        .interact_with
        .ok_or_else(|| io::Error::other("deposit preview has no smart account"))?;
    let transfer = transferCall::abi_decode(&transaction.data)?;
    if transaction.kind != ManagedPortfolioTransactionKind::Deposit
        || transaction.to != token
        || transaction.value != U256::ZERO
        || transfer.recipient != account
        || transfer.amount != raw_amount
        || transaction.gas_limit.is_some()
        || transaction.gas_price.is_some()
        || transaction.max_priority_fee_per_gas.is_some()
        || transaction.max_fee_per_gas.is_some()
    {
        return Err(
            io::Error::other("prepared deposit transaction does not match the request").into(),
        );
    }
    Ok(transaction)
}

async fn wait_for_deposit_reconciliation(
    gateway: &GliderAdapter,
    portfolio_id: &str,
    starting_value_usd: Decimal,
) -> Result<(), Box<dyn Error>> {
    let minimum_value_usd = starting_value_usd + Decimal::new(5, 1);
    for _ in 0..POSITION_ATTEMPTS {
        let positions = gateway.positions(portfolio_id).await?;
        if positions.total_value_usd >= minimum_value_usd {
            return Ok(());
        }
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }

    Err(io::Error::other("Glider did not reconcile the deposit before the polling deadline").into())
}

fn validate_prepared_withdrawal(
    prepared: &PreparedManagedPortfolioWithdrawal,
    portfolio_id: &str,
    recipient: WalletAddress,
    liquidate: bool,
) -> Result<(), Box<dyn Error>> {
    let recipient_address = prepared
        .typed_data
        .message
        .recipient_account_id
        .strip_prefix(&format!("eip155:{BNB_CHAIN_ID}:"))
        .and_then(|address| address.parse::<Address>().ok());
    if prepared.authorization_id.is_empty()
        || prepared.expires_at_ms <= chrono::Utc::now().timestamp_millis()
        || prepared.typed_data.primary_type != "Withdrawal"
        || prepared.typed_data.domain.chain_id != 56
        || prepared.typed_data.domain.verifying_contract.is_empty()
        || prepared.typed_data.message.portfolio_id != portfolio_id
        || recipient_address != Some(recipient.as_address())
        || prepared.typed_data.message.nonce.is_empty()
        || prepared.typed_data.message.expires_at <= 0
        || (prepared.typed_data.message.liquidate == Some(true)) != liquidate
        || prepared.typed_data.types.withdrawal.is_empty()
        || prepared.typed_data.types.withdrawal_asset.is_empty()
    {
        return Err(io::Error::other("Glider returned an invalid withdrawal authorization").into());
    }
    let _: TypedData = serde_json::from_value(serde_json::to_value(&prepared.typed_data)?)?;
    Ok(())
}

async fn sign_and_submit(
    gateway: &GliderAdapter,
    signer: &PrivateKeySigner,
    portfolio_id: &str,
    prepared: PreparedManagedPortfolioWithdrawal,
) -> Result<stockfront_core::managed_portfolios::SubmittedManagedPortfolioWithdrawal, Box<dyn Error>>
{
    let typed_data =
        serde_json::from_value::<TypedData>(serde_json::to_value(&prepared.typed_data)?)?;
    let signature = signer
        .sign_hash(&typed_data.eip712_signing_hash()?)
        .await?
        .to_string();
    Ok(gateway
        .submit_withdrawal(
            portfolio_id,
            &SubmitManagedPortfolioWithdrawalRequest {
                message: prepared.typed_data.message,
                signature,
            },
        )
        .await?)
}

async fn reject_failed_operation(
    gateway: &GliderAdapter,
    portfolio_id: &str,
    operation_id: &str,
) -> Result<(), Box<dyn Error>> {
    match gateway.operation(portfolio_id, operation_id).await {
        Ok(operation)
            if matches!(
                operation.state,
                ManagedPortfolioOperationState::Failed | ManagedPortfolioOperationState::Cancelled
            ) =>
        {
            Err(io::Error::other(
                operation
                    .error
                    .unwrap_or_else(|| "Glider withdrawal failed".to_owned()),
            )
            .into())
        }
        Ok(_) | Err(ManagedPortfolioProviderError::NotFound) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn wait_for_withdrawn_balance(
    gateway: &GliderAdapter,
    portfolio_id: &str,
    operation_id: &str,
    provider: &DynProvider,
    token: Address,
    wallet: Address,
    expected_balance: U256,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        if token_balance(provider, token, wallet).await? >= expected_balance {
            return Ok(());
        }
        reject_failed_operation(gateway, portfolio_id, operation_id).await?;
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(
        io::Error::other("withdrawn asset did not reach the wallet before the polling deadline")
            .into(),
    )
}

async fn wait_for_full_liquidation(
    gateway: &GliderAdapter,
    portfolio_id: &str,
    operation_id: &str,
    provider: &DynProvider,
    wallet: Address,
    starting_balance: U256,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..POSITION_ATTEMPTS {
        let positions = gateway.positions(portfolio_id).await?;
        let wallet_balance = token_balance(provider, BSC_USDT, wallet).await?;
        if positions.total_value_usd <= Decimal::ONE && wallet_balance > starting_balance {
            return Ok(());
        }
        reject_failed_operation(gateway, portfolio_id, operation_id).await?;
        tokio::time::sleep(POSITION_POLL_INTERVAL).await;
    }
    Err(io::Error::other(
        "Glider portfolio did not settle into the wallet before the polling deadline",
    )
    .into())
}

fn require_withdrawal_confirmation() -> Result<(), Box<dyn Error>> {
    if std::env::var("STOCKFRONT_GLIDER_WITHDRAW_CONFIRM")
        .ok()
        .as_deref()
        != Some(WITHDRAWAL_CONFIRMATION)
    {
        return Err(io::Error::other(format!(
            "set STOCKFRONT_GLIDER_WITHDRAW_CONFIRM={WITHDRAWAL_CONFIRMATION} to withdraw funds"
        ))
        .into());
    }
    Ok(())
}

fn live_signer() -> Result<PrivateKeySigner, Box<dyn Error>> {
    let private_key = Zeroizing::new(std::env::var("STOCKFRONT_E2E_PRIVATE_KEY")?);
    let signer = private_key.parse::<PrivateKeySigner>()?;
    drop(private_key);
    Ok(signer)
}

fn live_gateway() -> Result<GliderAdapter, Box<dyn Error>> {
    let api_key = std::env::var("GLIDER_API_KEY")?;
    let base_url = std::env::var("GLIDER_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.glider.fi/v2".to_owned());
    Ok(GliderAdapter::new(
        &base_url,
        api_key,
        Duration::from_secs(10),
    )?)
}

fn test_wallet() -> Result<WalletAddress, Box<dyn Error>> {
    Ok(WalletAddress::try_from(
        std::env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?,
    )?)
}
