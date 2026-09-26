use alloy_primitives::Address;
use binance_web3_rwa::models::{DefiPoolPosition, DefiPositionCollection, DefiPositionToken};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingAccount, BorrowingAsset, BorrowingPosition, BorrowingReadError, BorrowingToken,
        BorrowingVenue,
    },
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::client::VenusBorrowingReader;

const VENUS_PROTOCOL_ID: &str = "venus";

impl VenusBorrowingReader {
    pub(super) async fn read_binance_account(
        &self,
        wallet: WalletAddress,
    ) -> Result<BorrowingAccount, BorrowingReadError> {
        let snapshot = self.binance.defi_positions(wallet).await?;
        let mut positions = Vec::new();
        let mut total_supplied_usd = Decimal::ZERO;
        let mut total_borrowed_usd = Decimal::ZERO;
        let mut health_factor = None;

        for pool in snapshot
            .positions
            .address_list
            .unwrap_or_default()
            .into_iter()
            .flat_map(|address| address.protocol_list.unwrap_or_default())
            .filter(|protocol| protocol.defi_protocol_id.as_deref() == Some(VENUS_PROTOCOL_ID))
            .flat_map(|protocol| protocol.pool_list.unwrap_or_default())
        {
            let Some(read) = pool_position(pool)? else {
                continue;
            };
            total_supplied_usd = checked_total(total_supplied_usd, &read.position.supplied)?;
            total_borrowed_usd = checked_total(total_borrowed_usd, &read.position.borrowed)?;
            health_factor = minimum(health_factor, read.health_factor);
            positions.push(read.position);
        }

        Ok(BorrowingAccount {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            positions,
            total_supplied_usd,
            total_borrowed_usd,
            health_factor,
            observed_at_ms: snapshot.observed_at_ms,
        })
    }
}

fn pool_position(pool: DefiPoolPosition) -> Result<Option<PositionRead>, BorrowingReadError> {
    let Some(pool_address) = pool
        .pool_ca
        .as_deref()
        .and_then(|address| address.parse::<Address>().ok())
    else {
        return Ok(None);
    };

    let mut supplied = Vec::new();
    let mut borrowed = Vec::new();
    let mut health_factor = None;
    for collection in pool.position_collection_list.unwrap_or_default() {
        health_factor = minimum(health_factor, collection_health_factor(&collection)?);
        for position in collection.position_list.unwrap_or_default() {
            let Some(tokens) = position.token_list else {
                continue;
            };
            supplied.extend(
                tokens
                    .supply
                    .unwrap_or_default()
                    .into_iter()
                    .map(ProviderPositionToken::from)
                    .map(BorrowingAsset::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            );
            borrowed.extend(
                tokens
                    .borrow
                    .unwrap_or_default()
                    .into_iter()
                    .map(ProviderPositionToken::from)
                    .map(BorrowingAsset::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
    }

    if supplied.is_empty() && borrowed.is_empty() {
        return Ok(None);
    }

    Ok(Some(PositionRead {
        position: BorrowingPosition {
            pool: pool_address,
            supplied,
            borrowed,
        },
        health_factor,
    }))
}

struct PositionRead {
    position: BorrowingPosition,
    health_factor: Option<Decimal>,
}

struct ProviderPositionToken(DefiPositionToken);

impl From<DefiPositionToken> for ProviderPositionToken {
    fn from(value: DefiPositionToken) -> Self {
        Self(value)
    }
}

impl TryFrom<ProviderPositionToken> for BorrowingAsset {
    type Error = BorrowingReadError;

    fn try_from(value: ProviderPositionToken) -> Result<Self, Self::Error> {
        let value = value.0;
        Ok(Self {
            token: BorrowingToken {
                contract: required(value.token_address)?
                    .parse()
                    .map_err(|_| BorrowingReadError::InvalidData)?,
                symbol: required(value.token_symbol)?,
            },
            amount: parse_decimal(&required(value.token_amount)?)?,
            value_usd: parse_decimal(&required(value.token_value)?)?,
        })
    }
}

fn collection_health_factor(
    collection: &DefiPositionCollection,
) -> Result<Option<Decimal>, BorrowingReadError> {
    collection
        .position_collection_detail
        .as_ref()
        .and_then(Option::as_ref)
        .and_then(|detail| detail.health_factor.as_ref())
        .and_then(Option::as_ref)
        .map(|value| parse_decimal(value))
        .transpose()
}

fn checked_total(total: Decimal, assets: &[BorrowingAsset]) -> Result<Decimal, BorrowingReadError> {
    assets.iter().try_fold(total, |total, asset| {
        total
            .checked_add(asset.value_usd)
            .ok_or(BorrowingReadError::InvalidData)
    })
}

fn minimum(left: Option<Decimal>, right: Option<Decimal>) -> Option<Decimal> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    }
}

fn parse_decimal(value: &str) -> Result<Decimal, BorrowingReadError> {
    value.parse().map_err(|_| BorrowingReadError::InvalidData)
}

fn required<T>(value: Option<T>) -> Result<T, BorrowingReadError> {
    value.ok_or(BorrowingReadError::InvalidData)
}
