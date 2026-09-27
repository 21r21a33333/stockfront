use binance_web3_rwa::{
    apis::de_fi_data_api::{GetDefiPositionsParams, get_defi_positions},
    models::{DefiPositionToken, DefiPositions, GetDefiPositionsRequest},
};
use stockfront_core::{
    bstock_borrowing::{BorrowingAsset, BorrowingToken},
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::{
    client::BinanceWeb3Client,
    provider::{
        BinanceRequestError, InvalidProviderData, business_succeeded, map_request_error, parse,
        required,
    },
};

pub(crate) struct DefiPositionSnapshot {
    pub(crate) positions: DefiPositions,
    pub(crate) observed_at_ms: i64,
}

impl BinanceWeb3Client {
    pub(crate) async fn defi_positions<E>(
        &self,
        wallet: WalletAddress,
    ) -> Result<DefiPositionSnapshot, E>
    where
        E: From<BinanceRequestError> + From<InvalidProviderData>,
    {
        let mut request = GetDefiPositionsRequest::new(vec![wallet.as_address().to_string()]);
        request.binance_chain_ids = Some(vec![BNB_CHAIN_ID.to_string()]);
        let response = get_defi_positions(
            &self.configuration,
            GetDefiPositionsParams {
                get_defi_positions_request: request,
            },
        )
        .await
        .map_err(map_request_error)
        .map_err(E::from)?;
        if !business_succeeded(response.code, response.success) {
            return Err(E::from(BinanceRequestError::Unavailable));
        }

        Ok(DefiPositionSnapshot {
            positions: response
                .data
                .map(|positions| *positions)
                .ok_or(InvalidProviderData)
                .map_err(E::from)?,
            observed_at_ms: response
                .timestamp
                .ok_or(InvalidProviderData)
                .map_err(E::from)?,
        })
    }
}

pub(crate) struct ProviderPositionToken(pub(crate) DefiPositionToken);

impl From<DefiPositionToken> for ProviderPositionToken {
    fn from(value: DefiPositionToken) -> Self {
        Self(value)
    }
}

impl TryFrom<ProviderPositionToken> for BorrowingAsset {
    type Error = InvalidProviderData;

    fn try_from(value: ProviderPositionToken) -> Result<Self, Self::Error> {
        let value = value.0;
        Ok(Self {
            token: BorrowingToken {
                contract: parse(&required(value.token_address)?)?,
                symbol: required(value.token_symbol)?,
            },
            amount: parse(&required(value.token_amount)?)?,
            value_usd: parse(&required(value.token_value)?)?,
        })
    }
}
