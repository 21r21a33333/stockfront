mod support;

use std::{error::Error, sync::Arc};

use alloy_primitives::{Address, B256, U256};
use stockfront_core::{
    bstock_trade::{
        ApprovalRequest, BstockTradeGateway, BstockTradeService, PrepareTradeRequest,
        PreparedTrade, TradeExecutionMode, TradeGatewayError, TradePolicy, TradeQuoteRequest,
    },
    wallet_portfolio::WalletAddress,
};

use support::live_client;

const BINANCE_HOT_WALLET: &str = "0x8894e0a0c962cb723c1976a4421c95949be2d4e3";
const NVIDIA_BSTOCK: &str = "0x02fca66c1d1afb4e2a7884261eb00f63598a7436";
const USDT: &str = "0x55d398326f99059ff775485246999027b3197955";
const ONE_HUNDRED_USDT: &str = "100000000000000000000";

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn prepares_live_bstock_swap_from_binance() -> Result<(), Box<dyn Error>> {
    let wallet = BINANCE_HOT_WALLET.parse::<Address>()?;
    let bstock = NVIDIA_BSTOCK.parse::<Address>()?;
    let usdt = USDT.parse::<Address>()?;
    let amount = ONE_HUNDRED_USDT.parse::<U256>()?;
    let client = Arc::new(live_client()?);
    let policy = TradePolicy::new([usdt], 100, 50)?;
    let service = BstockTradeService::new(client.clone(), client, policy);
    let request = TradeQuoteRequest {
        wallet: WalletAddress::try_from(wallet)?,
        sell_token: usdt,
        buy_token: bstock,
        sell_amount: amount,
    };

    let quotes = service.quotes(&request).await?;

    assert!(
        !quotes.is_empty(),
        "Binance returned no executable bStock quote"
    );
    assert!(quotes.iter().all(|quote| {
        quote.sell.token == usdt
            && quote.sell.amount == amount
            && quote.buy.token == bstock
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
        .ok_or("Binance returned no SWAP route")?;
    let approval = service
        .prepare_approval(&ApprovalRequest {
            token: usdt,
            amount,
            venue: quote.venue,
        })
        .await?;
    assert_eq!(approval.to, usdt);
    assert_ne!(approval.spender, Address::ZERO);
    assert!(!approval.data.is_empty());

    let prepared = service
        .prepare_trade(&PrepareTradeRequest {
            trade: request,
            route_quote_id: quote.route_quote_id,
        })
        .await?;
    let PreparedTrade::Swap { transaction } = prepared else {
        return Err("Binance changed a SWAP quote into an RFQ payload".into());
    };
    assert_ne!(transaction.to, Address::ZERO);
    assert!(!transaction.data.is_empty());
    assert!(!transaction.gas_limit.is_zero());
    assert!(!transaction.min_receive_amount.is_zero());

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
