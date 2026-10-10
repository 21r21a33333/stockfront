mod support;
#[path = "support/trade.rs"]
mod trade_support;
#[path = "support/wallet.rs"]
mod wallet_support;

use std::{error::Error, io, sync::Arc, time::Duration};

use alloy_primitives::{Address, B256, U256, address};
use rust_decimal::Decimal;
use stockfront_adapters::binance_web3::BinanceWeb3Client;
use stockfront_core::bstock_trade::{
    ApprovalRequest, BstockTradeService, PrepareTradeRequest, PreparedTrade, SettlementPairs,
    TradeError, TradeExecutionMode, TradeGateway, TradeGatewayError, TradePolicy,
    TradeQuoteRequest, TradeTransactionState,
};

use support::live_client;
use trade_support::{NVIDIA_BSTOCK, USDT, live_trade_service};
use wallet_support::binance_hot_wallet;

const WBNB: Address = address!("bb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c");
const ONE_HUNDRED_USDT: &str = "100000000000000000000";
const ONE_HUNDRED_ONE_USDT: &str = "101000000000000000000";
const HALF_NVIDIA_BSTOCK: &str = "500000000000000000";
const QUOTE_EXPIRY_WAIT: Duration = Duration::from_secs(35);
const UNKNOWN_RFQ_ORDER_ID: &str = "00000000-0000-4000-8000-000000000000";
const SUCCESSFUL_NVIDIA_SALE_TX: &str =
    "0xd1f7d197516b6dccef6d85d768ce804a393633fb56e53c0697278a3bc58bcc3c";

fn offline_service(settlement_token: Address) -> Result<BstockTradeService, Box<dyn Error>> {
    let client = Arc::new(BinanceWeb3Client::new(
        "http://127.0.0.1:9",
        "unused",
        "unused",
        Duration::from_millis(1),
    )?);
    let policy = TradePolicy::new([settlement_token], 100, 50, SettlementPairs::Rejected)?;
    Ok(BstockTradeService::new(client.clone(), client, policy))
}

#[tokio::test]
async fn rejects_zero_trade_amount_before_calling_binance() -> Result<(), Box<dyn Error>> {
    let service = offline_service(USDT)?;

    let error = service
        .quotes(&TradeQuoteRequest {
            wallet: binance_hot_wallet()?,
            sell_token: USDT,
            buy_token: NVIDIA_BSTOCK,
            sell_amount: U256::ZERO,
        })
        .await
        .expect_err("zero-amount trade unexpectedly reached Binance");

    assert!(matches!(error, TradeError::InvalidAmount));
    Ok(())
}

#[tokio::test]
async fn rejects_unsupported_trade_pair_before_calling_binance() -> Result<(), Box<dyn Error>> {
    let service = offline_service(USDT)?;

    let error = service
        .quotes(&TradeQuoteRequest {
            wallet: binance_hot_wallet()?,
            sell_token: WBNB,
            buy_token: NVIDIA_BSTOCK,
            sell_amount: U256::from(1),
        })
        .await
        .expect_err("unsupported trade pair unexpectedly reached Binance");

    assert!(matches!(error, TradeError::UnsupportedTradePair));
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn prepares_live_bstock_swap_from_binance() -> Result<(), Box<dyn Error>> {
    assert_prepares_live_swap(USDT, NVIDIA_BSTOCK, ONE_HUNDRED_USDT.parse()?).await
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn prepares_live_bstock_sale_from_binance() -> Result<(), Box<dyn Error>> {
    assert_prepares_live_swap(NVIDIA_BSTOCK, USDT, HALF_NVIDIA_BSTOCK.parse()?).await
}

async fn assert_prepares_live_swap(
    sell_token: Address,
    buy_token: Address,
    amount: U256,
) -> Result<(), Box<dyn Error>> {
    let service = live_trade_service(USDT, 50)?;
    let request = TradeQuoteRequest {
        wallet: binance_hot_wallet()?,
        sell_token,
        buy_token,
        sell_amount: amount,
    };

    let quotes = service.quotes(&request).await?;
    assert!(
        !quotes.is_empty(),
        "Binance returned no executable bStock quote"
    );
    assert!(quotes.iter().all(|quote| {
        quote.sell.token == sell_token
            && quote.sell.amount == amount
            && quote.buy.token == buy_token
            && !quote.buy.amount.is_zero()
            && !quote.route_quote_id.is_empty()
            && !quote.venue.is_empty()
            && matches!(
                quote.execution_mode,
                TradeExecutionMode::Swap | TradeExecutionMode::Rfq
            )
    }));
    let quote = quotes
        .into_iter()
        .find(|quote| matches!(quote.execution_mode, TradeExecutionMode::Swap))
        .ok_or_else(|| io::Error::other("Binance returned no SWAP route"))?;

    let approval = service
        .prepare_approval(&ApprovalRequest {
            token: sell_token,
            amount,
            venue: quote.venue,
        })
        .await?;
    assert_eq!(approval.to, sell_token);
    assert_ne!(approval.spender, Address::ZERO);
    assert!(!approval.data.is_empty());

    let prepared = service
        .prepare_trade(&PrepareTradeRequest {
            trade: request,
            route_quote_id: quote.route_quote_id,
        })
        .await?;
    let PreparedTrade::Swap { transaction } = prepared else {
        return Err(io::Error::other("Binance changed a SWAP quote into an RFQ payload").into());
    };
    assert_ne!(transaction.to, Address::ZERO);
    assert!(!transaction.data.is_empty());
    assert!(!transaction.gas_limit.is_zero());
    assert!(!transaction.min_receive_amount.is_zero());

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials and waits for quote expiry"]
async fn rejects_expired_live_bstock_quote() -> Result<(), Box<dyn Error>> {
    let amount = ONE_HUNDRED_USDT.parse::<U256>()?;
    let service = live_trade_service(USDT, 50)?;
    let request = TradeQuoteRequest {
        wallet: binance_hot_wallet()?,
        sell_token: USDT,
        buy_token: NVIDIA_BSTOCK,
        sell_amount: amount,
    };
    let quote = service
        .quotes(&request)
        .await?
        .into_iter()
        .find(|quote| matches!(quote.execution_mode, TradeExecutionMode::Swap))
        .ok_or("Binance returned no SWAP route")?;

    tokio::time::sleep(QUOTE_EXPIRY_WAIT).await;

    let error = service
        .prepare_trade(&PrepareTradeRequest {
            trade: request,
            route_quote_id: quote.route_quote_id,
        })
        .await
        .expect_err("Binance unexpectedly accepted an expired quote");
    assert!(matches!(
        error,
        TradeError::Gateway(TradeGatewayError::QuoteExpired)
    ));

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn rejects_mismatched_live_bstock_quote() -> Result<(), Box<dyn Error>> {
    let quoted_amount = ONE_HUNDRED_USDT.parse::<U256>()?;
    let changed_amount = ONE_HUNDRED_ONE_USDT.parse::<U256>()?;
    let service = live_trade_service(USDT, 50)?;
    let quote_request = TradeQuoteRequest {
        wallet: binance_hot_wallet()?,
        sell_token: USDT,
        buy_token: NVIDIA_BSTOCK,
        sell_amount: quoted_amount,
    };
    let quote = service
        .quotes(&quote_request)
        .await?
        .into_iter()
        .find(|quote| matches!(quote.execution_mode, TradeExecutionMode::Swap))
        .ok_or("Binance returned no SWAP route")?;

    let error = service
        .prepare_trade(&PrepareTradeRequest {
            trade: TradeQuoteRequest {
                sell_amount: changed_amount,
                ..quote_request
            },
            route_quote_id: quote.route_quote_id,
        })
        .await
        .expect_err("Binance unexpectedly accepted a modified quote amount");
    assert!(matches!(
        error,
        TradeError::Gateway(TradeGatewayError::QuoteMismatch)
    ));

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn reports_unknown_live_swap_transaction() -> Result<(), Box<dyn Error>> {
    let error = live_client()?
        .transaction_status(B256::ZERO)
        .await
        .expect_err("zero transaction hash unexpectedly exists");

    assert!(matches!(error, TradeGatewayError::TransactionNotFound));
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn reads_successful_live_bstock_transaction() -> Result<(), Box<dyn Error>> {
    let transaction_hash = SUCCESSFUL_NVIDIA_SALE_TX.parse::<B256>()?;

    let status = live_client()?.transaction_status(transaction_hash).await?;

    assert!(matches!(status.state, TradeTransactionState::Succeeded));
    assert!(
        status
            .sold
            .iter()
            .any(|amount| amount.token == NVIDIA_BSTOCK && amount.amount > Decimal::ZERO)
    );
    assert!(
        status
            .bought
            .iter()
            .any(|amount| amount.token == USDT && amount.amount > Decimal::ZERO)
    );
    assert!(status.block_number.is_some());
    assert!(status.executed_at_ms.is_some());
    Ok(())
}

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn reports_unknown_live_rfq_order() -> Result<(), Box<dyn Error>> {
    let error = live_client()?
        .order_status(UNKNOWN_RFQ_ORDER_ID)
        .await
        .expect_err("nonexistent RFQ order unexpectedly exists");

    assert!(matches!(error, TradeGatewayError::OrderNotFound));
    Ok(())
}
