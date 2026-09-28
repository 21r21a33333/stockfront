pub(super) mod borrowing;
pub(super) mod bstocks;
pub(super) mod carry;
pub(super) mod trades;
pub(super) mod wallets;

use stockfront_core::{bstock_market::Address, wallet_portfolio::WalletAddress};

use super::error::ApiError;

pub(super) fn parse_wallet(value: &str) -> Result<WalletAddress, ApiError> {
    let address = value
        .parse::<Address>()
        .map_err(|_| ApiError::INVALID_WALLET_ADDRESS)?;
    WalletAddress::try_from(address).map_err(Into::into)
}
