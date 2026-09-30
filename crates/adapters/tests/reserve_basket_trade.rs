#[path = "support/erc20_allowance.rs"]
mod erc20_allowance_support;
#[path = "support/erc20.rs"]
mod erc20_support;
#[path = "support/live_evm.rs"]
mod live_evm_support;

use std::{error::Error, io, sync::Arc, time::Duration};

use alloy::{
    dyn_abi::TypedData,
    primitives::{Address, U256, address},
    signers::{Signer, local::PrivateKeySigner},
};
use stockfront_adapters::reserve::ReserveAdapter;
use stockfront_core::{
    baskets::BasketService,
    bstock_trade::{
        ApprovalRequest, PrepareTradeRequest, PreparedTrade, RfqVenue, SubmitOrderRequest,
        TradeError, TradeExecutionMode, TradeGateway, TradeGatewayError, TradeOrderState,
        TradeOrderStatus, TradePolicy, TradeQuote, TradeQuoteRequest,
    },
    wallet_portfolio::WalletAddress,
};
use zeroize::Zeroizing;

use erc20_allowance_support::token_allowance;
use erc20_support::token_balance;
use live_evm_support::{LiveEvmContext, live_evm_context, send_transaction};

const USDT: Address = address!("55d398326f99059ff775485246999027b3197955");
const PHOTON: Address = address!("a0fe4e0aeca5479705ce996615b2eacb6b6a10fb");
const DEFAULT_WALLET: Address = address!("4b393bec5dee11799f698440ab19c0fbfe15d74c");
const DEFAULT_ROUND_TRIP_USDT: &str = "1000000000000000000";
const FUSION_PREPARE_ATTEMPTS: usize = 3;
const FUSION_SETTLEMENT_ATTEMPTS: usize = 120;
const FUSION_SETTLEMENT_INTERVAL: Duration = Duration::from_secs(2);

struct LiveBasketContext {
    evm: LiveEvmContext,
    service: BasketService,
    signer: PrivateKeySigner,
}

#[tokio::test]
#[ignore = "spends real USDT and BNB on BSC mainnet"]
async fn buys_and_sells_reserve_basket_on_bsc() -> Result<(), Box<dyn Error>> {
    let context = live_basket_context().await?;
    let wallet = context.evm.wallet;
    let usdt_amount = std::env::var("STOCKFRONT_BASKET_TEST_USDT_AMOUNT")
        .unwrap_or_else(|_| DEFAULT_ROUND_TRIP_USDT.to_owned())
        .parse::<U256>()?;
    if usdt_amount.is_zero() {
        return Err(io::Error::other("basket test USDT amount must be greater than zero").into());
    }

    let starting_usdt = token_balance(&context.evm.provider, USDT, wallet.as_address()).await?;
    if starting_usdt < usdt_amount {
        return Err(io::Error::other(format!(
            "wallet needs at least {usdt_amount} raw USDT units"
        ))
        .into());
    }
    let starting_basket = token_balance(&context.evm.provider, PHOTON, wallet.as_address()).await?;

    let purchase = execute_fusion_trade(&context, USDT, PHOTON, usdt_amount).await?;
    assert_filled(&purchase, "basket purchase")?;
    let usdt_after_purchase =
        token_balance(&context.evm.provider, USDT, wallet.as_address()).await?;
    let basket_after_purchase =
        token_balance(&context.evm.provider, PHOTON, wallet.as_address()).await?;
    if usdt_after_purchase >= starting_usdt {
        return Err(io::Error::other("basket purchase did not spend USDT").into());
    }
    let purchased_basket = basket_after_purchase
        .checked_sub(starting_basket)
        .filter(|amount| !amount.is_zero())
        .ok_or_else(|| io::Error::other("basket purchase did not increase the PHOTON balance"))?;

    let sale = execute_fusion_trade(&context, PHOTON, USDT, purchased_basket).await?;
    assert_filled(&sale, "basket sale")?;
    let final_usdt = token_balance(&context.evm.provider, USDT, wallet.as_address()).await?;
    let final_basket = token_balance(&context.evm.provider, PHOTON, wallet.as_address()).await?;
    if final_usdt <= usdt_after_purchase {
        return Err(io::Error::other("basket sale did not return USDT to the wallet").into());
    }
    if final_basket != starting_basket {
        return Err(io::Error::other("round trip left an unexpected PHOTON balance").into());
    }
    Ok(())
}

async fn live_basket_context() -> Result<LiveBasketContext, Box<dyn Error>> {
    let evm = live_evm_context().await?;
    let private_key = Zeroizing::new(std::env::var("STOCKFRONT_E2E_PRIVATE_KEY")?);
    let signer = private_key.parse::<PrivateKeySigner>()?;
    drop(private_key);
    if signer.address() != evm.wallet.as_address() {
        return Err(io::Error::other("transaction and order signers do not match").into());
    }
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let adapter = Arc::new(ReserveAdapter::new(&base_url, Duration::from_secs(10), 50)?);
    let service = BasketService::new(
        adapter.clone(),
        adapter,
        TradePolicy::new([USDT], 10_000, 50)?,
    );
    Ok(LiveBasketContext {
        evm,
        service,
        signer,
    })
}

async fn execute_fusion_trade(
    context: &LiveBasketContext,
    sell_token: Address,
    buy_token: Address,
    sell_amount: U256,
) -> Result<TradeOrderStatus, Box<dyn Error>> {
    let quote = fusion_quote(
        &context.service,
        context.evm.wallet,
        sell_token,
        buy_token,
        sell_amount,
    )
    .await?;
    ensure_allowance(context, sell_token, sell_amount, &quote).await?;

    let prepared = prepare_fusion_trade(
        &context.service,
        context.evm.wallet,
        sell_token,
        buy_token,
        sell_amount,
    )
    .await?;
    let PreparedTrade::Rfq {
        submission_id,
        route_quote_id,
        venue,
        typed_data,
    } = prepared
    else {
        return Err(
            io::Error::other("Reserve prepared a transaction instead of a Fusion order").into(),
        );
    };
    if !matches!(venue, RfqVenue::InchFusion) {
        return Err(io::Error::other("Reserve prepared an unexpected RFQ venue").into());
    }

    let typed_data = serde_json::from_str::<TypedData>(&typed_data)?;
    let signature = context
        .signer
        .sign_hash(&typed_data.eip712_signing_hash()?)
        .await?
        .to_string();
    let submitted = context
        .service
        .submit_order(&SubmitOrderRequest {
            submission_id,
            route_quote_id,
            venue,
            signature,
        })
        .await?;
    if !matches!(submitted.state, TradeOrderState::PendingVendor) {
        return Err(
            io::Error::other("Reserve returned an unexpected submitted-order state").into(),
        );
    }
    wait_for_fusion_order(&context.service, &submitted.order_id).await
}

async fn ensure_allowance(
    context: &LiveBasketContext,
    token: Address,
    amount: U256,
    quote: &TradeQuote,
) -> Result<(), Box<dyn Error>> {
    let Some(spender) = quote.approval_target else {
        return Ok(());
    };
    let wallet = context.evm.wallet.as_address();
    if token_allowance(&context.evm.provider, token, wallet, spender).await? >= amount {
        return Ok(());
    }

    let approval = context
        .service
        .prepare_approval(&ApprovalRequest {
            token,
            amount,
            venue: quote.venue.clone(),
        })
        .await?;
    if approval.to != token || approval.spender != spender {
        return Err(io::Error::other(
            "Reserve approval does not match its quoted allowance target",
        )
        .into());
    }
    send_transaction(
        &context.evm.provider,
        approval.to,
        approval.data,
        U256::ZERO,
    )
    .await?;
    if token_allowance(&context.evm.provider, token, wallet, spender).await? < amount {
        return Err(io::Error::other(
            "approval transaction did not establish the required allowance",
        )
        .into());
    }
    Ok(())
}

async fn prepare_fusion_trade(
    service: &BasketService,
    wallet: WalletAddress,
    sell_token: Address,
    buy_token: Address,
    sell_amount: U256,
) -> Result<PreparedTrade, Box<dyn Error>> {
    for attempt in 0..FUSION_PREPARE_ATTEMPTS {
        let quote = fusion_quote(service, wallet, sell_token, buy_token, sell_amount).await?;
        let request = PrepareTradeRequest {
            trade: trade_request(wallet, sell_token, buy_token, sell_amount),
            route_quote_id: quote.route_quote_id,
        };
        match service.prepare_trade(&request).await {
            Ok(prepared) => return Ok(prepared),
            Err(TradeError::Gateway(TradeGatewayError::QuoteMismatch))
                if attempt + 1 < FUSION_PREPARE_ATTEMPTS => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(
        io::Error::other("Reserve quote moved beyond tolerance on every preparation attempt")
            .into(),
    )
}

async fn fusion_quote(
    service: &BasketService,
    wallet: WalletAddress,
    sell_token: Address,
    buy_token: Address,
    sell_amount: U256,
) -> Result<TradeQuote, Box<dyn Error>> {
    let quote = service
        .quotes(&trade_request(wallet, sell_token, buy_token, sell_amount))
        .await?
        .into_iter()
        .find(|quote| {
            quote.venue == "InchFusion" && matches!(quote.execution_mode, TradeExecutionMode::Rfq)
        })
        .ok_or_else(|| io::Error::other("Reserve returned no executable Fusion quote"))?;
    if quote.sell.token != sell_token
        || quote.sell.amount != sell_amount
        || quote.buy.token != buy_token
        || quote.buy.amount.is_zero()
    {
        return Err(
            io::Error::other("Reserve Fusion quote does not match the requested trade").into(),
        );
    }
    Ok(quote)
}

fn trade_request(
    wallet: WalletAddress,
    sell_token: Address,
    buy_token: Address,
    sell_amount: U256,
) -> TradeQuoteRequest {
    TradeQuoteRequest {
        wallet,
        sell_token,
        buy_token,
        sell_amount,
    }
}

async fn wait_for_fusion_order(
    service: &BasketService,
    order_id: &str,
) -> Result<TradeOrderStatus, Box<dyn Error>> {
    for _ in 0..FUSION_SETTLEMENT_ATTEMPTS {
        match service.order_status(order_id).await {
            Ok(status) if matches!(status.state, TradeOrderState::Filled) => return Ok(status),
            Ok(status)
                if matches!(
                    status.state,
                    TradeOrderState::Failed | TradeOrderState::Expired | TradeOrderState::Cancelled
                ) =>
            {
                return Err(io::Error::other(format!(
                    "Reserve Fusion order ended as {:?}",
                    status.state
                ))
                .into());
            }
            Ok(_)
            | Err(TradeError::Gateway(
                TradeGatewayError::OrderNotFound | TradeGatewayError::Unavailable,
            )) => {
                tokio::time::sleep(FUSION_SETTLEMENT_INTERVAL).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::other("Reserve Fusion order settlement timed out").into())
}

fn assert_filled(status: &TradeOrderStatus, action: &str) -> Result<(), Box<dyn Error>> {
    if !matches!(status.state, TradeOrderState::Filled)
        || status.transaction_hash.is_none()
        || status
            .filled_buy_amount
            .is_none_or(|amount| amount.is_zero())
    {
        return Err(io::Error::other(format!(
            "{action} lacks complete on-chain settlement evidence"
        ))
        .into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "calls the live Reserve 1inch Fusion API"]
async fn quotes_and_prepares_live_bnb_basket_trades() -> Result<(), Box<dyn Error>> {
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let wallet = std::env::var("STOCKFRONT_TEST_WALLET")
        .ok()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(DEFAULT_WALLET);
    let wallet = WalletAddress::try_from(wallet)?;
    let adapter = Arc::new(ReserveAdapter::new(&base_url, Duration::from_secs(10), 50)?);
    let service = BasketService::new(
        adapter.clone(),
        adapter,
        TradePolicy::new([USDT], 10_000, 50)?,
    );
    let buy_amount = U256::from(1_000_000_000_000_000_000_u128);
    let quote = fusion_quote(&service, wallet, USDT, PHOTON, buy_amount).await?;

    assert_eq!(quote.venue, "InchFusion");
    assert!(matches!(quote.execution_mode, TradeExecutionMode::Rfq));
    assert_eq!(quote.sell.token, USDT);
    assert_eq!(quote.buy.token, PHOTON);
    assert!(quote.buy.amount > U256::ZERO);
    assert!(quote.price_impact_percent.is_none());

    let approval = service
        .prepare_approval(&ApprovalRequest {
            token: USDT,
            amount: buy_amount,
            venue: quote.venue.clone(),
        })
        .await?;
    assert_eq!(approval.to, USDT);
    assert_eq!(
        approval.spender,
        address!("111111125421ca6dc452d289314280a0f8842a65")
    );
    assert!(approval.gas_limit.is_none());
    assert!(approval.gas_price.is_none());

    let prepared = prepare_fusion_trade(&service, wallet, USDT, PHOTON, buy_amount).await?;
    let PreparedTrade::Rfq {
        venue, typed_data, ..
    } = prepared
    else {
        return Err("Reserve prepared a transaction instead of a Fusion order".into());
    };
    assert!(matches!(venue, RfqVenue::InchFusion));
    let typed_data: serde_json::Value = serde_json::from_str(&typed_data)?;
    assert_eq!(typed_data["domain"]["chainId"], 56);
    assert_eq!(typed_data["primaryType"], "Order");

    let sale_amount = U256::from(10_000_000_000_000_000_u64);
    let sale_quote = fusion_quote(&service, wallet, PHOTON, USDT, sale_amount).await?;
    assert_eq!(sale_quote.sell.token, PHOTON);
    assert_eq!(sale_quote.buy.token, USDT);
    assert!(matches!(sale_quote.execution_mode, TradeExecutionMode::Rfq));
    assert!(matches!(
        prepare_fusion_trade(&service, wallet, PHOTON, USDT, sale_amount).await?,
        PreparedTrade::Rfq {
            venue: RfqVenue::InchFusion,
            ..
        }
    ));
    Ok(())
}

#[tokio::test]
#[ignore = "calls the live Reserve 1inch Fusion API"]
async fn reports_unknown_live_basket_order() -> Result<(), Box<dyn Error>> {
    let base_url = std::env::var("RESERVE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.reserve.org".to_owned());
    let adapter = ReserveAdapter::new(&base_url, Duration::from_secs(10), 50)?;

    let error = adapter
        .order_status("0x0000000000000000000000000000000000000000000000000000000000000000")
        .await
        .expect_err("unknown Fusion order unexpectedly exists");

    assert!(matches!(error, TradeGatewayError::OrderNotFound));
    Ok(())
}
