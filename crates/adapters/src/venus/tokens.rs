use alloy_primitives::Address;
use stockfront_core::bstock_trade::SettlementTokenMetadata;

use super::{VenusBorrowingAdapter, VenusConfigurationError, contracts::IERC20};

impl VenusBorrowingAdapter {
    /// Reads symbol and decimals from the configured settlement-token contracts.
    ///
    /// # Errors
    ///
    /// Returns [`VenusConfigurationError::Unavailable`] when a token cannot be read.
    pub async fn settlement_token_metadata(
        &self,
        tokens: &[Address],
    ) -> Result<Vec<SettlementTokenMetadata>, VenusConfigurationError> {
        let mut metadata = Vec::with_capacity(tokens.len());
        for &address in tokens {
            let token = IERC20::new(address, &self.provider);
            let (symbol, decimals) =
                tokio::try_join!(async { token.symbol().call().await }, async {
                    token.decimals().call().await
                })
                .map_err(|_| VenusConfigurationError::Unavailable)?;
            metadata.push(SettlementTokenMetadata {
                address,
                symbol,
                decimals,
            });
        }
        Ok(metadata)
    }
}
