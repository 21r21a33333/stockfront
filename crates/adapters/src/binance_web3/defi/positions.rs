use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingAsset,
    bstock_market::BNB_CHAIN_ID,
    carry::{CarryAccount, CarryPosition, EarnProviderError},
    wallet_portfolio::WalletAddress,
};

use super::super::{
    client::BinanceWeb3Client,
    positions::{DefiPositionSnapshot, ProviderPositionToken},
    provider::{InvalidProviderData, parse, required, required_non_empty},
};

impl BinanceWeb3Client {
    pub(super) async fn earn_positions(
        &self,
        wallet: WalletAddress,
        reviewed_investment_ids: &[String],
    ) -> Result<CarryAccount, EarnProviderError> {
        let snapshot = self.defi_positions::<EarnProviderError>(wallet).await?;
        carry_account(snapshot, reviewed_investment_ids).map_err(Into::into)
    }
}

fn carry_account(
    snapshot: DefiPositionSnapshot,
    reviewed_investment_ids: &[String],
) -> Result<CarryAccount, InvalidProviderData> {
    let mut positions = Vec::new();
    let mut total_value_usd = Decimal::ZERO;

    for protocol in snapshot
        .positions
        .address_list
        .unwrap_or_default()
        .into_iter()
        .flat_map(|address| address.protocol_list.unwrap_or_default())
    {
        let protocol_id = protocol.defi_protocol_id;
        let protocol_name = protocol.protocol_name;
        for position in protocol
            .pool_list
            .unwrap_or_default()
            .into_iter()
            .flat_map(|pool| pool.position_collection_list.unwrap_or_default())
            .flat_map(|collection| collection.position_list.unwrap_or_default())
        {
            let Some(investment_id) = reviewed_investment_ids.iter().find(|reviewed| {
                position
                    .investment_ids
                    .as_deref()
                    .unwrap_or_default()
                    .contains(reviewed)
            }) else {
                continue;
            };
            let position = CarryPosition::try_from(ProviderCarryPosition {
                position,
                investment_id: investment_id.clone(),
                protocol_id: protocol_id.clone(),
                protocol_name: protocol_name.clone(),
            })?;
            total_value_usd = total_value_usd
                .checked_add(position.value_usd)
                .ok_or(InvalidProviderData)?;
            positions.push(position);
        }
    }

    Ok(CarryAccount {
        chain_id: BNB_CHAIN_ID,
        positions,
        total_value_usd,
        observed_at_ms: snapshot.observed_at_ms,
    })
}

struct ProviderCarryPosition {
    position: models::DefiPosition,
    investment_id: String,
    protocol_id: Option<String>,
    protocol_name: Option<String>,
}

impl TryFrom<ProviderCarryPosition> for CarryPosition {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderCarryPosition) -> Result<Self, Self::Error> {
        let tokens = required(provider.position.token_list)?;
        let assets = tokens
            .supply
            .unwrap_or_default()
            .into_iter()
            .map(ProviderPositionToken::from)
            .map(BorrowingAsset::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if assets.is_empty() {
            return Err(InvalidProviderData);
        }

        Ok(Self {
            position_id: required_non_empty(provider.position.position_id)?,
            investment_id: provider.investment_id,
            protocol_id: required_non_empty(provider.protocol_id)?,
            protocol_name: required_non_empty(provider.protocol_name)?,
            value_usd: parse(&required(provider.position.position_value)?)?,
            assets,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn selects_only_reviewed_positions_and_totals_their_value() {
        let snapshot = DefiPositionSnapshot {
            positions: models::DefiPositions {
                total_value: Some("75".to_owned()),
                address_list: Some(vec![models::DefiAddressPosition {
                    protocol_list: Some(vec![models::DefiProtocolPosition {
                        binance_chain_id: Some(BNB_CHAIN_ID.to_string()),
                        defi_protocol_id: Some("lista".to_owned()),
                        protocol_name: Some("Lista DAO".to_owned()),
                        protocol_total_value: Some("75".to_owned()),
                        pool_list: Some(vec![models::DefiPoolPosition {
                            pool_ca: None,
                            position_collection_list: Some(vec![models::DefiPositionCollection {
                                position_collection_detail: None,
                                position_list: Some(vec![
                                    position("reviewed", "25"),
                                    position("unrelated", "50"),
                                ]),
                            }]),
                        }]),
                    }]),
                }]),
            },
            observed_at_ms: 1_000,
        };

        let account = carry_account(snapshot, &["reviewed".to_owned()])
            .unwrap_or_else(|_| panic!("valid position fixture was rejected"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].investment_id, "reviewed");
        assert_eq!(account.total_value_usd, Decimal::new(25, 0));
        assert_eq!(account.observed_at_ms, 1_000);
    }

    fn position(investment_id: &str, value: &str) -> models::DefiPosition {
        models::DefiPosition {
            position_id: Some(format!("position-{investment_id}")),
            investment_ids: Some(vec![investment_id.to_owned()]),
            position_value: Some(value.to_owned()),
            token_list: Some(Box::new(models::DefiPositionTokens {
                supply: Some(vec![models::DefiPositionToken {
                    token_address: Some(
                        address!("55d398326f99059fF775485246999027B3197955").to_string(),
                    ),
                    token_symbol: Some("USDT".to_owned()),
                    token_amount: Some(value.to_owned()),
                    token_value: Some(value.to_owned()),
                }]),
                borrow: None,
            })),
        }
    }
}
