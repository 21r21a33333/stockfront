use std::{env, error::Error, sync::Arc, time::Duration};

use alloy_primitives::address;
use stockfront_adapters::venus::VenusBorrowingAdapter;

use crate::support::live_client;

const DEFAULT_BSC_RPC_URL: &str = "https://bsc-dataseed.bnbchain.org";
const DEFAULT_VENUS_API_URL: &str = "https://api.venus.io";
const VENUS_COMPTROLLER: alloy_primitives::Address =
    address!("fD36E2c2a6789Db23113685031d7F16329158384");

pub async fn live_venus_adapter() -> Result<VenusBorrowingAdapter, Box<dyn Error>> {
    let rpc_url = env::var("BSC_RPC_URL").unwrap_or_else(|_| DEFAULT_BSC_RPC_URL.to_owned());
    let venus_api_url =
        env::var("VENUS_API_BASE_URL").unwrap_or_else(|_| DEFAULT_VENUS_API_URL.to_owned());
    Ok(VenusBorrowingAdapter::new(
        &venus_api_url,
        &rpc_url,
        VENUS_COMPTROLLER,
        Arc::new(live_client()?),
        Duration::from_secs(15),
    )
    .await?)
}
