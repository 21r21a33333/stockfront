use std::{env, error::Error, io};

use alloy::{
    network::{EthereumWallet, TransactionBuilder},
    primitives::{Address, B256, Bytes, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use stockfront_core::{bstock_market::BNB_CHAIN_ID, wallet_portfolio::WalletAddress};
use zeroize::Zeroizing;

const DEFAULT_BSC_RPC_URL: &str = "https://bsc-dataseed.bnbchain.org";
const MINIMUM_BNB_FOR_GAS: &str = "500000000000000";
const LIVE_CONFIRMATION_ENV: &str = "STOCKFRONT_E2E_CONFIRM";
const LIVE_CONFIRMATION: &str = "I_UNDERSTAND_THIS_SPENDS_REAL_FUNDS";
const GAS_HEADROOM_PERCENT: u64 = 25;

pub struct LiveEvmContext {
    pub provider: DynProvider,
    pub wallet: WalletAddress,
}

pub async fn live_evm_context() -> Result<LiveEvmContext, Box<dyn Error>> {
    if env::var(LIVE_CONFIRMATION_ENV).ok().as_deref() != Some(LIVE_CONFIRMATION) {
        return Err(io::Error::other(format!(
            "set {LIVE_CONFIRMATION_ENV}={LIVE_CONFIRMATION} to arm the live-money test"
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

    let wallet = WalletAddress::try_from(wallet_address)?;
    let minimum_gas_balance = MINIMUM_BNB_FOR_GAS.parse::<U256>()?;
    if provider.get_balance(wallet.as_address()).await? < minimum_gas_balance {
        return Err(io::Error::other("wallet needs at least 0.0005 BNB for gas").into());
    }
    Ok(LiveEvmContext { provider, wallet })
}

pub async fn send_transaction(
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
