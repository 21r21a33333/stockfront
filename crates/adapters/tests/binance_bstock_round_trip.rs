mod support;
#[path = "support/trade.rs"]
mod trade_support;

use std::{env, error::Error, io, time::Duration};

use alloy::{
    network::{EthereumWallet, TransactionBuilder},
    primitives::{Address, B256, Bytes, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol,
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_market::BNB_CHAIN_ID,
    bstock_trade::{
        ApprovalRequest, BstockTradeError, BstockTradeService, PrepareTradeRequest, PreparedTrade,
        TradeExecutionMode, TradeGatewayError, TradeQuote, TradeQuoteRequest,
        TradeTransactionState, TradeTransactionStatus,
    },
    wallet_portfolio::WalletAddress,
};
use zeroize::Zeroizing;

use trade_support::{NVIDIA_BSTOCK, USDT, live_trade_service};

const DEFAULT_BSC_RPC_URL: &str = "https://bsc-dataseed.bnbchain.org";
const TESLA_BSTOCK: Address =
    alloy::primitives::address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const SIX_USDT: &str = "6000000000000000000";
const MINIMUM_BNB_FOR_GAS: &str = "500000000000000";
const LIVE_TRADE_CONFIRMATION_ENV: &str = "STOCKFRONT_E2E_CONFIRM";
const LIVE_TRADE_CONFIRMATION: &str = "I_UNDERSTAND_THIS_SPENDS_REAL_FUNDS";
const ROUND_TRIP_SLIPPAGE_BPS: u32 = 100;
const GAS_HEADROOM_PERCENT: u64 = 25;
const SETTLEMENT_ATTEMPTS: usize = 30;
const SETTLEMENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

sol! {
    #[sol(rpc)]
    interface Erc20 {
        function balanceOf(address owner) external view returns (uint256 balance);
    }
}

struct PurchasedBstock {
    amount: U256,
    transaction_hash: B256,
}

struct LiveTradeContext {
    provider: DynProvider,
    service: BstockTradeService,
    wallet: WalletAddress,
}

#[tokio::test]
#[ignore = "spends 6 USDT and BNB on BSC mainnet and keeps the purchased TSLAB"]
async fn buys_tesla_bstock_for_borrowing_on_bsc() -> Result<(), Box<dyn Error>> {
    let context = live_trade_context().await?;
    let buy_amount = SIX_USDT.parse::<U256>()?;
    if token_balance(&context.provider, USDT, context.wallet.as_address()).await? < buy_amount {
        return Err(io::Error::other("wallet needs at least 6 USDT").into());
    }

    let starting_balance =
        token_balance(&context.provider, TESLA_BSTOCK, context.wallet.as_address()).await?;
    let purchase = buy_bstock(
        &context.provider,
        &context.service,
        context.wallet,
        USDT,
        TESLA_BSTOCK,
        buy_amount,
    )
    .await?;
    let final_balance =
        token_balance(&context.provider, TESLA_BSTOCK, context.wallet.as_address()).await?;
    if final_balance <= starting_balance {
        return Err(io::Error::other("purchase did not increase the TSLAB balance").into());
    }

    let status = wait_for_settlement(&context.service, purchase.transaction_hash).await?;
    assert_settlement_direction(&status, USDT, TESLA_BSTOCK)
}

#[tokio::test]
#[ignore = "spends real USDT and BNB on BSC mainnet"]
async fn buys_and_sells_nvidia_bstock_on_bsc() -> Result<(), Box<dyn Error>> {
    let context = live_trade_context().await?;
    let buy_amount = SIX_USDT.parse::<U256>()?;
    let starting_usdt = token_balance(&context.provider, USDT, context.wallet.as_address()).await?;
    if starting_usdt < buy_amount {
        return Err(io::Error::other("wallet needs at least 6 USDT").into());
    }
    let starting_bstock = token_balance(
        &context.provider,
        NVIDIA_BSTOCK,
        context.wallet.as_address(),
    )
    .await?;
    let purchase = buy_bstock(
        &context.provider,
        &context.service,
        context.wallet,
        USDT,
        NVIDIA_BSTOCK,
        buy_amount,
    )
    .await?;
    let usdt_after_buy =
        token_balance(&context.provider, USDT, context.wallet.as_address()).await?;
    let sale_transaction_hash = sell_bstock(
        &context.provider,
        &context.service,
        context.wallet,
        NVIDIA_BSTOCK,
        USDT,
        purchase.amount,
    )
    .await?;

    let final_bstock = token_balance(
        &context.provider,
        NVIDIA_BSTOCK,
        context.wallet.as_address(),
    )
    .await?;
    if final_bstock != starting_bstock {
        return Err(io::Error::other("round trip left an unexpected NVDAB balance").into());
    }

    let final_usdt = token_balance(&context.provider, USDT, context.wallet.as_address()).await?;
    if final_usdt <= usdt_after_buy {
        return Err(io::Error::other("sale did not return USDT to the wallet").into());
    }

    let purchase_status = wait_for_settlement(&context.service, purchase.transaction_hash).await?;
    assert_settlement_direction(&purchase_status, USDT, NVIDIA_BSTOCK)?;
    let sale_status = wait_for_settlement(&context.service, sale_transaction_hash).await?;
    assert_settlement_direction(&sale_status, NVIDIA_BSTOCK, USDT)
}

async fn live_trade_context() -> Result<LiveTradeContext, Box<dyn Error>> {
    if env::var(LIVE_TRADE_CONFIRMATION_ENV).ok().as_deref() != Some(LIVE_TRADE_CONFIRMATION) {
        return Err(io::Error::other(format!(
            "set {LIVE_TRADE_CONFIRMATION_ENV}={LIVE_TRADE_CONFIRMATION} to arm the live-money test"
        ))
        .into());
    }

    let private_key = Zeroizing::new(env::var("STOCKFRONT_E2E_PRIVATE_KEY")?);
    let signer = private_key.parse::<PrivateKeySigner>()?;
    drop(private_key);

    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);
    let rpc_url = env::var("BSC_RPC_URL").unwrap_or_else(|_| DEFAULT_BSC_RPC_URL.to_owned());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(&rpc_url)
        .await?
        .erased();
    if provider.get_chain_id().await? != BNB_CHAIN_ID {
        return Err(io::Error::other("BSC RPC did not report chain ID 56").into());
    }

    let minimum_gas_balance = MINIMUM_BNB_FOR_GAS.parse::<U256>()?;
    let wallet_address = WalletAddress::try_from(wallet_address)?;
    let service = live_trade_service(USDT, ROUND_TRIP_SLIPPAGE_BPS)?;

    if provider.get_balance(wallet_address.as_address()).await? < minimum_gas_balance {
        return Err(io::Error::other("wallet needs at least 0.0005 BNB for gas").into());
    }
    Ok(LiveTradeContext {
        provider,
        service,
        wallet: wallet_address,
    })
}

async fn buy_bstock(
    provider: &impl Provider,
    service: &BstockTradeService,
    wallet: WalletAddress,
    usdt: Address,
    bstock: Address,
    amount: U256,
) -> Result<PurchasedBstock, Box<dyn Error>> {
    let starting_balance = token_balance(provider, bstock, wallet.as_address()).await?;
    let transaction_hash = execute_swap(
        provider,
        service,
        TradeQuoteRequest {
            wallet,
            sell_token: usdt,
            buy_token: bstock,
            sell_amount: amount,
        },
    )
    .await?;

    let balance = token_balance(provider, bstock, wallet.as_address()).await?;
    let bought = balance
        .checked_sub(starting_balance)
        .ok_or_else(|| io::Error::other("bStock balance did not increase after purchase"))?;
    if bought.is_zero() {
        return Err(io::Error::other("purchase received zero bStock").into());
    }
    Ok(PurchasedBstock {
        amount: bought,
        transaction_hash,
    })
}

async fn sell_bstock(
    provider: &impl Provider,
    service: &BstockTradeService,
    wallet: WalletAddress,
    bstock: Address,
    usdt: Address,
    amount: U256,
) -> Result<B256, Box<dyn Error>> {
    execute_swap(
        provider,
        service,
        TradeQuoteRequest {
            wallet,
            sell_token: bstock,
            buy_token: usdt,
            sell_amount: amount,
        },
    )
    .await
}

async fn execute_swap(
    provider: &impl Provider,
    service: &BstockTradeService,
    request: TradeQuoteRequest,
) -> Result<B256, Box<dyn Error>> {
    let approval_quote = swap_quote(service.quotes(&request).await?, None, None)?;
    let venue = approval_quote.venue;
    let approval = service
        .prepare_approval(&ApprovalRequest {
            token: request.sell_token,
            amount: request.sell_amount,
            venue: venue.clone(),
        })
        .await?;
    send_transaction(provider, approval.to, approval.data, U256::ZERO).await?;

    let quote = swap_quote(
        service.quotes(&request).await?,
        Some(&venue),
        Some(approval.spender),
    )?;
    let prepared = service
        .prepare_trade(&PrepareTradeRequest {
            trade: request,
            route_quote_id: quote.route_quote_id,
        })
        .await?;
    let PreparedTrade::Swap { transaction } = prepared else {
        return Err(io::Error::other("Binance changed the selected route to RFQ").into());
    };
    send_transaction(
        provider,
        transaction.to,
        transaction.data,
        transaction.value,
    )
    .await
}

fn swap_quote(
    quotes: Vec<TradeQuote>,
    required_venue: Option<&str>,
    required_approval_target: Option<Address>,
) -> Result<TradeQuote, Box<dyn Error>> {
    quotes
        .into_iter()
        .find(|quote| {
            matches!(quote.execution_mode, TradeExecutionMode::Swap)
                && required_venue.is_none_or(|venue| quote.venue == venue)
                && required_approval_target
                    .is_none_or(|spender| quote.approval_target == Some(spender))
        })
        .ok_or_else(|| io::Error::other("Binance returned no usable SWAP route").into())
}

async fn token_balance(
    provider: &impl Provider,
    token: Address,
    owner: Address,
) -> Result<U256, Box<dyn Error>> {
    Ok(Erc20::new(token, provider).balanceOf(owner).call().await?)
}

async fn send_transaction(
    provider: &impl Provider,
    to: Address,
    input: Bytes,
    value: U256,
) -> Result<B256, Box<dyn Error>> {
    let mut request = TransactionRequest::default()
        .with_to(to)
        .with_input(input)
        .with_value(value)
        .with_gas_price(provider.get_gas_price().await?);
    let estimated_gas = provider.estimate_gas(request.clone()).await?;
    let gas_limit = estimated_gas
        .checked_mul(100 + GAS_HEADROOM_PERCENT)
        .and_then(|gas| gas.checked_div(100))
        .ok_or_else(|| io::Error::other("gas-limit calculation overflowed"))?;
    request.set_gas_limit(gas_limit);

    let receipt = provider
        .send_transaction(request)
        .await?
        .get_receipt()
        .await?;
    if !receipt.status() {
        return Err(
            io::Error::other(format!("transaction {} reverted", receipt.transaction_hash)).into(),
        );
    }
    Ok(receipt.transaction_hash)
}

async fn wait_for_settlement(
    service: &BstockTradeService,
    transaction_hash: B256,
) -> Result<TradeTransactionStatus, Box<dyn Error>> {
    for _ in 0..SETTLEMENT_ATTEMPTS {
        match service.transaction_status(transaction_hash).await {
            Ok(status) if matches!(status.state, TradeTransactionState::Succeeded) => {
                return Ok(status);
            }
            Ok(status) if matches!(status.state, TradeTransactionState::Failed) => {
                let reason = status
                    .error
                    .unwrap_or_else(|| "Binance reported failed settlement".to_owned());
                return Err(io::Error::other(reason).into());
            }
            Ok(_)
            | Err(BstockTradeError::Gateway(
                TradeGatewayError::TransactionNotFound | TradeGatewayError::Unavailable,
            )) => tokio::time::sleep(SETTLEMENT_POLL_INTERVAL).await,
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::other("Binance settlement status timed out").into())
}

fn assert_settlement_direction(
    status: &TradeTransactionStatus,
    sold_token: Address,
    bought_token: Address,
) -> Result<(), Box<dyn Error>> {
    let sold = status
        .sold
        .iter()
        .any(|amount| amount.token == sold_token && amount.amount > Decimal::ZERO);
    let bought = status
        .bought
        .iter()
        .any(|amount| amount.token == bought_token && amount.amount > Decimal::ZERO);
    if !sold || !bought {
        return Err(
            io::Error::other("Binance settlement amounts do not match the trade pair").into(),
        );
    }
    Ok(())
}
