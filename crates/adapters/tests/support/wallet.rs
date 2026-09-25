use alloy_primitives::{Address, address};
use stockfront_core::wallet_portfolio::{WalletAddress, WalletAddressError};

const BINANCE_HOT_WALLET: Address = address!("8894e0a0c962cb723c1976a4421c95949be2d4e3");

pub fn binance_hot_wallet() -> Result<WalletAddress, WalletAddressError> {
    WalletAddress::try_from(BINANCE_HOT_WALLET)
}
