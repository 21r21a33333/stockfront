use binance_web3_rwa::{
    apis::de_fi_data_api::{GetDefiPositionsParams, get_defi_positions},
    models::{DefiPositions, GetDefiPositionsRequest},
};
use stockfront_core::{
    bstock_borrowing::BorrowingReadError, bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::client::{BinanceWeb3Client, is_business_success};

pub(crate) struct DefiPositionSnapshot {
    pub(crate) positions: DefiPositions,
    pub(crate) observed_at_ms: i64,
}

impl BinanceWeb3Client {
    pub(crate) async fn defi_positions(
        &self,
        wallet: WalletAddress,
    ) -> Result<DefiPositionSnapshot, BorrowingReadError> {
        let mut request = GetDefiPositionsRequest::new(vec![wallet.as_address().to_string()]);
        request.binance_chain_ids = Some(vec![BNB_CHAIN_ID.to_string()]);
        let response = get_defi_positions(
            &self.configuration,
            GetDefiPositionsParams {
                get_defi_positions_request: request,
            },
        )
        .await
        .map_err(|_| BorrowingReadError::Unavailable)?;

        if !is_business_success(response.code, response.success) {
            return Err(BorrowingReadError::Unavailable);
        }

        Ok(DefiPositionSnapshot {
            positions: response
                .data
                .map(|positions| *positions)
                .ok_or(BorrowingReadError::InvalidData)?,
            observed_at_ms: response.timestamp.ok_or(BorrowingReadError::InvalidData)?,
        })
    }
}
