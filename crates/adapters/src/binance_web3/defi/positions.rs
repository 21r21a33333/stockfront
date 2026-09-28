use std::collections::HashSet;

use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingAsset,
    bstock_market::BNB_CHAIN_ID,
    carry::{CarryAccount, CarryPosition, CarryRedemption, EarnProviderError},
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
            let candidate = ProviderCarryPosition {
                position,
                investment_ids,
                protocol_id: protocol_id.clone(),
                protocol_name: protocol_name.clone(),
            };
            let Some(position) = candidate.try_into_position()? else {
                continue;
            };
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

impl ProviderCarryPosition {
    fn try_into_position(self) -> Result<Option<CarryPosition>, InvalidProviderData> {
        let models::DefiPosition {
            position_id,
            position_value,
            token_list,
            position_detail,
            ..
        } = self.position;
        let supplied_tokens = token_list
            .and_then(|tokens| tokens.supply)
            .unwrap_or_default();
        if supplied_tokens.is_empty() {
            return Ok(None);
        }
        if self.investment_ids.is_empty() {
            return Err(InvalidProviderData);
        }
        let redemption = position_detail
            .flatten()
            .and_then(|detail| CarryRedemption::try_from(ProviderCarryRedemption(*detail)).ok());
        let mut assets = supplied_tokens
            .into_iter()
            .map(ProviderPositionToken::from)
            .map(BorrowingAsset::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if assets
            .iter()
            .any(|asset| asset.amount < Decimal::ZERO || asset.value_usd < Decimal::ZERO)
        {
            return Err(InvalidProviderData);
        }
        assets.retain(|asset| !asset.amount.is_zero());
        if assets.is_empty() {
            return Ok(None);
        }

        Ok(Some(CarryPosition {
            position_id: required_non_empty(position_id)?,
            investment_ids: self.investment_ids,
            protocol_id: required_non_empty(self.protocol_id)?,
            protocol_name: required_non_empty(self.protocol_name)?,
            value_usd: parse(&required(position_value)?)?,
            assets,
            redemption,
        }))
    }
}

struct ProviderCarryRedemption(models::DefiPositionDetail);

impl TryFrom<ProviderCarryRedemption> for CarryRedemption {
    type Error = InvalidProviderData;

    fn try_from(provider: ProviderCarryRedemption) -> Result<Self, Self::Error> {
        let detail = provider.0;
        let id = required_non_empty(detail.redemption_id.flatten())?;
        let claimable = detail
            .claim_types
            .unwrap_or_default()
            .iter()
            .any(|claim_type| claim_type == "REDEMPTION");
        Ok(Self { id, claimable })
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn keeps_all_ids_for_each_resolved_earn_position() {
        let snapshot = snapshot(vec![
            position(&["base", "reward"], "25"),
            position(&["unrelated"], "50"),
        ]);

        let account = carry_account(snapshot, &HashSet::from(["base".to_owned()]))
            .unwrap_or_else(|_| panic!("valid position fixture was rejected"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].investment_ids, ["base", "reward"]);
        assert_eq!(account.positions[0].redemption, None);
        assert_eq!(account.total_value_usd, Decimal::new(25, 0));
        assert_eq!(account.observed_at_ms, 1_000);
    }

    #[test]
    fn maps_pending_and_claimable_redemptions_from_position_metadata() {
        let mut pending = position(&["pending"], "25");
        pending.position_detail = Some(Some(Box::new(position_detail("pending-redemption", &[]))));
        let mut claimable = position(&["claimable"], "50");
        claimable.position_detail = Some(Some(Box::new(position_detail(
            "claimable-redemption",
            &["REWARD_PROTOCOL", "REDEMPTION"],
        ))));
        let snapshot = snapshot(vec![pending, claimable]);

        let account = carry_account(
            snapshot,
            &HashSet::from(["pending".to_owned(), "claimable".to_owned()]),
        )
        .unwrap_or_else(|_| panic!("valid redemption metadata was rejected"));

        assert_eq!(
            account.positions[0].redemption,
            Some(CarryRedemption {
                id: "pending-redemption".to_owned(),
                claimable: false,
            })
        );
        assert_eq!(
            account.positions[1].redemption,
            Some(CarryRedemption {
                id: "claimable-redemption".to_owned(),
                claimable: true,
            })
        );
    }

    #[test]
    fn excludes_borrow_only_records_from_carry_positions() {
        let account = carry_account(
            snapshot(vec![
                position(&["deposit"], "25"),
                borrow_position("borrow", "10"),
            ]),
            &HashSet::from(["deposit".to_owned(), "borrow".to_owned()]),
        )
        .unwrap_or_else(|_| panic!("borrow-only record invalidated carry positions"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].investment_ids, ["deposit"]);
        assert_eq!(account.total_value_usd, Decimal::new(25, 0));
    }

    #[test]
    fn excludes_stale_zero_positions_without_dropping_nonzero_dust() {
        let mut redeemed = position(&["redeemed"], "0.00000438");
        let redeemed_asset = redeemed
            .token_list
            .as_deref_mut()
            .and_then(|tokens| tokens.supply.as_deref_mut())
            .and_then(|assets| assets.first_mut())
            .unwrap_or_else(|| panic!("position fixture has no supplied token"));
        redeemed_asset.token_amount = Some("0.0".to_owned());
        redeemed_asset.token_value = Some("0.00000000".to_owned());

        let account = carry_account(
            snapshot(vec![redeemed, position(&["dust"], "0.00000191")]),
            &HashSet::from(["redeemed".to_owned(), "dust".to_owned()]),
        )
        .unwrap_or_else(|_| panic!("valid dust position was rejected"));

        assert_eq!(account.positions.len(), 1);
        assert_eq!(account.positions[0].investment_ids, ["dust"]);
        assert_eq!(account.total_value_usd, Decimal::new(191, 8));
    }

    #[test]
    fn rejects_malformed_supplied_records() {
        let mut malformed = position(&["deposit"], "25");
        let supplied = malformed
            .token_list
            .as_deref_mut()
            .and_then(|tokens| tokens.supply.as_deref_mut())
            .unwrap_or_else(|| panic!("position fixture has no supplied token"));
        supplied[0].token_amount = None;

        let result = carry_account(
            snapshot(vec![malformed]),
            &HashSet::from(["deposit".to_owned()]),
        );

        assert!(matches!(result, Err(InvalidProviderData)));
    }

    fn snapshot(positions: Vec<models::DefiPosition>) -> DefiPositionSnapshot {
        DefiPositionSnapshot {
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
                                position_list: Some(positions),
                            }]),
                        }]),
                    }]),
                }]),
            },
            observed_at_ms: 1_000,
        }
    }

    fn position_detail(redemption_id: &str, claim_types: &[&str]) -> models::DefiPositionDetail {
        models::DefiPositionDetail {
            claim_types: Some(
                claim_types
                    .iter()
                    .map(|claim_type| (*claim_type).to_owned())
                    .collect(),
            ),
            redemption_id: Some(Some(redemption_id.to_owned())),
        }
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
            position_detail: None,
        }
    }

    fn borrow_position(investment_id: &str, value: &str) -> models::DefiPosition {
        let mut position = position(&[investment_id], value);
        let tokens = position
            .token_list
            .as_deref_mut()
            .unwrap_or_else(|| panic!("position fixture has no tokens"));
        tokens.borrow = tokens.supply.take();
        position
    }
}
