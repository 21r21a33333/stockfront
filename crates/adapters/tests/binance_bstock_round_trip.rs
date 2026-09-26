#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;
mod support;
#[path = "support/trade.rs"]
mod trade_support;

use std::{error::Error, io, time::Duration};

use alloy::{
    primitives::{Address, B256, U256},
    providers::{DynProvider, Provider},
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_trade::{
        ApprovalRequest, BstockTradeError, BstockTradeService, PrepareTradeRequest, PreparedTrade,
        TradeExecutionMode, TradeGatewayError, TradeQuote, TradeQuoteRequest,
        TradeTransactionState, TradeTransactionStatus,
    },
    wallet_portfolio::WalletAddress,
};

use erc20_support::token_balance;
use live_evm_support::{live_evm_context, send_transaction};
use trade_support::{NVIDIA_BSTOCK, USDT, live_trade_service};

const TESLA_BSTOCK: Address =
    alloy::primitives::address!("5b1910eAaD6450E50f816082Aa078C41F10C292f");
const SIX_USDT: &str = "6000000000000000000";
const ROUND_TRIP_SLIPPAGE_BPS: u32 = 100;
const SETTLEMENT_ATTEMPTS: usize = 30;
const SETTLEMENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

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
    let context = live_evm_context().await?;
    let service = live_trade_service(USDT, ROUND_TRIP_SLIPPAGE_BPS)?;
    Ok(LiveTradeContext {
        provider: context.provider,
        service,
        wallet: context.wallet,
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
