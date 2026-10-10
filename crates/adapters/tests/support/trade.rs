use std::{error::Error, sync::Arc};

use alloy_primitives::{Address, address};
use stockfront_core::bstock_trade::{BstockTradeService, SettlementPairs, TradePolicy};

use crate::support::live_client;

pub const NVIDIA_BSTOCK: Address = address!("02fca66c1d1afb4e2a7884261eb00f63598a7436");
pub const USDT: Address = address!("55d398326f99059ff775485246999027b3197955");

pub fn live_trade_service(
    settlement_token: Address,
    slippage_bps: u32,
) -> Result<BstockTradeService, Box<dyn Error>> {
    let client = Arc::new(live_client()?);
    let policy = TradePolicy::new(
        [settlement_token],
        100,
        slippage_bps,
        SettlementPairs::Rejected,
    )?;
    Ok(BstockTradeService::new(client.clone(), client, policy))
}
