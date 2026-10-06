mod support;
#[path = "support/wallet.rs"]
mod wallet_support;

use std::{error::Error, io, sync::Arc};

use alloy_primitives::{Address, U256, address};
use stockfront_core::{
    bstock_market::{BNB_CHAIN_ID, BstockMarketService},
    bstock_trade::{BstockTradeService, PreparedTrade, TradeExecutionMode, TradePolicy},
    recipes::{
        PrepareRecipeRequest, PreparedRecipeStep, RecipeContext, RecipeDefinition, RecipeEdge,
        RecipeNode, RecipeNodePreview, RecipeNodeSelection, RecipePortRef, RecipePreviewRequest,
        RecipePrimitive, RecipeService,
    },
};
use uuid::Uuid;

use support::live_client;
use wallet_support::binance_hot_wallet;

const NVIDIA_BSTOCK: Address = address!("02fca66c1d1afb4e2a7884261eb00f63598a7436");
const USDT: Address = address!("55d398326f99059ff775485246999027b3197955");
const SIX_USDT: &str = "6000000000000000000";

#[tokio::test]
#[ignore = "requires live Binance Web3 credentials"]
async fn previews_and_prepares_live_bstock_recipe() -> Result<(), Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let market = Arc::new(BstockMarketService::new(client.clone()));
    let trades = Arc::new(BstockTradeService::new(
        client.clone(),
        client,
        TradePolicy::new([USDT], 100, 50)?,
    ));
    let service = RecipeService::new(market, trades, vec![USDT]);

    let catalog = service.catalog().await?;
    assert_eq!(catalog.primitives.len(), 2);
    assert_eq!(catalog.settlement_tokens, vec![USDT]);
    assert!(
        catalog
            .bstocks
            .iter()
            .any(|asset| asset.id.contract == NVIDIA_BSTOCK)
    );

    let wallet = binance_hot_wallet()?;
    let recipe = recipe(SIX_USDT.parse()?);
    let preview = service
        .preview(&RecipePreviewRequest {
            recipe: recipe.clone(),
            context: RecipeContext {
                wallet,
                completed_nodes: Vec::new(),
            },
        })
        .await?;
    let Some(RecipeNodePreview::Trade { routes, .. }) = preview.nodes.into_iter().next() else {
        return Err(io::Error::other("recipe preview did not contain a trade").into());
    };
    let quote = routes
        .into_iter()
        .find(|quote| matches!(quote.execution_mode, TradeExecutionMode::Swap))
        .ok_or_else(|| io::Error::other("Binance returned no SWAP recipe route"))?;
    assert_eq!(quote.sell.token, USDT);
    assert_eq!(quote.sell.amount, SIX_USDT.parse::<U256>()?);
    assert_eq!(quote.buy.token, NVIDIA_BSTOCK);
    assert!(!quote.buy.amount.is_zero());
    assert!(!quote.route_quote_id.is_empty());
    assert!(!quote.venue.is_empty());

    let prepared = service
        .prepare(&PrepareRecipeRequest {
            recipe,
            context: RecipeContext {
                wallet,
                completed_nodes: Vec::new(),
            },
            selections: vec![RecipeNodeSelection::Trade {
                node_id: Uuid::from_u128(2),
                route_quote_id: quote.route_quote_id,
                venue: quote.venue,
            }],
        })
        .await?;
    let [
        PreparedRecipeStep::Approval {
            node_id: approval_node,
            action: approval,
        },
        PreparedRecipeStep::Trade {
            node_id: trade_node,
            action: PreparedTrade::Swap { transaction },
        },
    ] = prepared.steps.as_slice()
    else {
        return Err(io::Error::other("recipe actions were not approval then SWAP").into());
    };
    assert_eq!(approval_node, trade_node);
    assert_eq!(approval.to, USDT);
    assert_ne!(approval.spender, Address::ZERO);
    assert!(!approval.data.is_empty());
    assert_ne!(transaction.to, Address::ZERO);
    assert!(!transaction.data.is_empty());
    assert!(!transaction.gas_limit.is_zero());
    assert!(!transaction.min_receive_amount.is_zero());
    Ok(())
}

fn recipe(raw_amount: U256) -> RecipeDefinition {
    let source_id = Uuid::from_u128(1);
    let trade_id = Uuid::from_u128(2);
    RecipeDefinition {
        schema_version: 1,
        chain_id: BNB_CHAIN_ID,
        nodes: vec![
            RecipeNode {
                id: source_id,
                primitive: RecipePrimitive::WalletAsset {
                    token: USDT,
                    raw_amount,
                },
            },
            RecipeNode {
                id: trade_id,
                primitive: RecipePrimitive::Trade {
                    buy_token: NVIDIA_BSTOCK,
                },
            },
        ],
        edges: vec![RecipeEdge {
            id: Uuid::from_u128(3),
            from: RecipePortRef {
                node_id: source_id,
                port: "asset".to_owned(),
            },
            to: RecipePortRef {
                node_id: trade_id,
                port: "sell".to_owned(),
            },
        }],
    }
}
