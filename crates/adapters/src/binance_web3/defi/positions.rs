use std::collections::HashSet;

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
    ) -> Result<CarryAccount, EarnProviderError> {
        let snapshot = self.defi_positions::<EarnProviderError>(wallet).await?;
        let mut earn_investment_ids = HashSet::new();
        for investment_id in investment_ids(&snapshot) {
            if self.is_bsc_earn_investment(&investment_id).await? {
                earn_investment_ids.insert(investment_id);
            }
        }
        carry_account(snapshot, &earn_investment_ids).map_err(Into::into)
    }
}

fn investment_ids(snapshot: &DefiPositionSnapshot) -> HashSet<String> {
    snapshot
        .positions
        .address_list
        .as_deref()
        .unwrap_or_default()
        .iter()
        .flat_map(|address| address.protocol_list.as_deref().unwrap_or_default())
        .flat_map(|protocol| protocol.pool_list.as_deref().unwrap_or_default())
        .flat_map(|pool| pool.position_collection_list.as_deref().unwrap_or_default())
        .flat_map(|collection| collection.position_list.as_deref().unwrap_or_default())
        .flat_map(|position| position.investment_ids.as_deref().unwrap_or_default())
        .filter(|investment_id| !investment_id.is_empty())
        .cloned()
        .collect()
}

fn carry_account(
    snapshot: DefiPositionSnapshot,
    earn_investment_ids: &HashSet<String>,
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
            let investment_ids = position
                .investment_ids
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|investment_id| !investment_id.is_empty())
                .cloned()
                .collect::<Vec<_>>();
            if !investment_ids
                .iter()
                .any(|investment_id| earn_investment_ids.contains(investment_id))
            {
                continue;
            }
            let position = CarryPosition::try_from(ProviderCarryPosition {
                position,
                investment_ids,
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
    investment_ids: Vec<String>,
    protocol_id: Option<String>,
    protocol_name: Option<String>,
}

impl TryFrom<ProviderCarryPosition> for CarryPosition {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderCarryPosition) -> Result<Self, Self::Error> {
        if provider.investment_ids.is_empty() {
            return Err(InvalidProviderData);
        }
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
            investment_ids: provider.investment_ids,
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
    fn keeps_all_ids_for_each_resolved_earn_position() {
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
                                    position(&["base", "reward"], "25"),
                                    position(&["unrelated"], "50"),
                                ]),
                            }]),
                        }]),
                    }]),
                }]),
            },
            observed_at_ms: 1_000,
        };

        let account = carry_account(snapshot, &HashSet::from(["base".to_owned()]))
            .unwrap_or_else(|_| panic!("valid position fixture was rejected"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].investment_ids, ["base", "reward"]);
        assert_eq!(account.total_value_usd, Decimal::new(25, 0));
        assert_eq!(account.observed_at_ms, 1_000);
    }

    fn position(investment_ids: &[&str], value: &str) -> models::DefiPosition {
        models::DefiPosition {
            position_id: Some(format!("position-{}", investment_ids[0])),
            investment_ids: Some(
                investment_ids
                    .iter()
                    .map(|investment_id| (*investment_id).to_owned())
                    .collect(),
            ),
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
