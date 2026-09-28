use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::BorrowingAsset,
    bstock_market::BNB_CHAIN_ID,
    liquidity::{
        LiquidityAccount, LiquidityClaimAction, LiquidityPosition, LiquidityProviderError,
    },
    wallet_portfolio::WalletAddress,
};

use super::super::{
    client::BinanceWeb3Client,
    positions::{DefiPositionSnapshot, ProviderPositionToken},
    provider::{InvalidProviderData, parse, required, required_non_empty},
};

impl BinanceWeb3Client {
    pub(super) async fn liquidity_positions(
        &self,
        wallet: WalletAddress,
    ) -> Result<LiquidityAccount, LiquidityProviderError> {
        let snapshot = self
            .defi_positions::<LiquidityProviderError>(wallet)
            .await?;
        liquidity_account(snapshot).map_err(Into::into)
    }
}

fn liquidity_account(
    snapshot: DefiPositionSnapshot,
) -> Result<LiquidityAccount, InvalidProviderData> {
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
        for pool in protocol.pool_list.unwrap_or_default() {
            if pool.pool_type.as_deref() != Some("Liquidity Pool") {
                continue;
            }
            let pool_address = pool.pool_ca;
            let pool_detail = pool.pool_detail.flatten().map(|detail| *detail);
            for position in pool
                .position_collection_list
                .unwrap_or_default()
                .into_iter()
                .flat_map(|collection| collection.position_list.unwrap_or_default())
            {
                let candidate = ProviderLiquidityPosition {
                    position,
                    protocol_id: protocol_id.clone(),
                    protocol_name: protocol_name.clone(),
                    pool_address: pool_address.clone(),
                    pool_detail: pool_detail.clone(),
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
    }

    Ok(LiquidityAccount {
        chain_id: BNB_CHAIN_ID,
        positions,
        total_value_usd,
        observed_at_ms: snapshot.observed_at_ms,
    })
}

struct ProviderLiquidityPosition {
    position: models::DefiPosition,
    protocol_id: Option<String>,
    protocol_name: Option<String>,
    pool_address: Option<String>,
    pool_detail: Option<models::DefiPoolDetail>,
}

impl ProviderLiquidityPosition {
    fn try_into_position(self) -> Result<Option<LiquidityPosition>, InvalidProviderData> {
        let models::DefiPosition {
            position_id,
            investment_ids,
            position_value,
            token_list,
            position_detail,
        } = self.position;
        let Some(detail) = position_detail.flatten().map(|detail| *detail) else {
            return Ok(None);
        };
        let (Some(nft_id), Some(tick_lower), Some(tick_upper), Some(in_range)) = (
            detail.nft_id,
            detail.tick_lower,
            detail.tick_upper,
            detail.active,
        ) else {
            return Ok(None);
        };
        let tokens = token_list.unwrap_or_default();
        let assets = map_assets(tokens.supply.unwrap_or_default())?;
        if assets.is_empty() {
            return Ok(None);
        }
        let claimable_assets = tokens
            .reward
            .unwrap_or_default()
            .into_iter()
            .chain(tokens.farming_reward.unwrap_or_default())
            .filter_map(|token| BorrowingAsset::try_from(ProviderPositionToken(token)).ok())
            .filter(|asset| asset.amount > Decimal::ZERO)
            .collect();
        let investment_ids = investment_ids
            .unwrap_or_default()
            .into_iter()
            .filter(|investment_id| !investment_id.is_empty())
            .collect::<Vec<_>>();
        if investment_ids.is_empty() {
            return Err(InvalidProviderData);
        }
        let value_usd = parse(&required(position_value)?)?;
        if value_usd < Decimal::ZERO {
            return Err(InvalidProviderData);
        }

        Ok(Some(LiquidityPosition {
            position_id: required_non_empty(position_id)?,
            investment_ids,
            protocol_id: required_non_empty(self.protocol_id)?,
            protocol_name: required_non_empty(self.protocol_name)?,
            pool_address: parse(&required_non_empty(self.pool_address)?)?,
            nft_id: required_non_empty(Some(nft_id))?,
            value_usd,
            assets,
            claimable_assets,
            fee_tier: self
                .pool_detail
                .as_ref()
                .and_then(|detail| detail.fee_tier.as_deref())
                .and_then(|fee| parse(fee).ok()),
            current_tick: self
                .pool_detail
                .as_ref()
                .and_then(|detail| detail.tick.as_deref())
                .and_then(|tick| parse(tick).ok()),
            lower_tick: parse(&tick_lower)?,
            upper_tick: parse(&tick_upper)?,
            in_range,
            claim_actions: claim_actions(detail.claim_types.unwrap_or_default()),
        }))
    }
}

fn map_assets(
    tokens: Vec<models::DefiPositionToken>,
) -> Result<Vec<BorrowingAsset>, InvalidProviderData> {
    let mut assets = tokens
        .into_iter()
        .map(ProviderPositionToken)
        .map(BorrowingAsset::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    if assets
        .iter()
        .any(|asset| asset.amount < Decimal::ZERO || asset.value_usd < Decimal::ZERO)
    {
        return Err(InvalidProviderData);
    }
    assets.retain(|asset| !asset.amount.is_zero());
    Ok(assets)
}

fn claim_actions(claim_types: Vec<String>) -> Vec<LiquidityClaimAction> {
    let mut actions = Vec::new();
    for claim_type in claim_types {
        let action = match claim_type.as_str() {
            "LP_FEE" => LiquidityClaimAction::LpFee,
            "REWARD_INVESTMENT" => LiquidityClaimAction::RewardInvestment,
            _ => continue,
        };
        if !actions.contains(&action) {
            actions.push(action);
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn maps_concentrated_liquidity_position_and_claim_metadata() {
        let account = liquidity_account(snapshot())
            .unwrap_or_else(|_| panic!("valid liquidity position fixture was rejected"));

        assert_eq!(account.positions.len(), 1);
        let position = &account.positions[0];
        assert_eq!(position.investment_ids, ["investment"]);
        assert_eq!(
            position.pool_address,
            address!("46A15B0b27311cedF172AB29E4f4766fbE7F4364")
        );
        assert_eq!(position.nft_id, "123");
        assert_eq!(position.value_usd, Decimal::new(71, 2));
        assert_eq!(position.assets.len(), 2);
        assert_eq!(position.claimable_assets.len(), 1);
        assert_eq!(position.fee_tier, Some(2_500));
        assert_eq!(position.current_tick, Some(-58_900));
        assert_eq!(position.lower_tick, -59_500);
        assert_eq!(position.upper_tick, -58_450);
        assert!(position.in_range);
        assert_eq!(
            position.claim_actions,
            [
                LiquidityClaimAction::LpFee,
                LiquidityClaimAction::RewardInvestment
            ]
        );
        assert_eq!(account.total_value_usd, Decimal::new(71, 2));
        assert_eq!(account.observed_at_ms, 1_000);
    }

    #[test]
    fn excludes_zero_balance_assets_from_stale_positions() {
        let assets = map_assets(vec![token(
            "5b1910eAaD6450E50f816082Aa078C41F10C292f",
            "TSLAB",
            "0",
            "0",
        )])
        .unwrap_or_else(|_| panic!("zero-balance provider asset was invalid"));

        assert!(assets.is_empty());
    }

    fn snapshot() -> DefiPositionSnapshot {
        let mut detail = models::DefiPositionDetail::new();
        detail.nft_id = Some("123".to_owned());
        detail.tick_lower = Some("-59500".to_owned());
        detail.tick_upper = Some("-58450".to_owned());
        detail.active = Some(true);
        detail.claim_types = Some(vec![
            "LP_FEE".to_owned(),
            "UNKNOWN".to_owned(),
            "REWARD_INVESTMENT".to_owned(),
        ]);

        let mut tokens = models::DefiPositionTokens::new();
        tokens.supply = Some(vec![
            token(
                "55d398326f99059ff775485246999027b3197955",
                "USDT",
                "0.35",
                "0.35",
            ),
            token(
                "5b1910eAaD6450E50f816082Aa078C41F10C292f",
                "TSLAB",
                "0.001",
                "0.36",
            ),
        ]);
        tokens.reward = Some(vec![token(
            "55d398326f99059ff775485246999027b3197955",
            "USDT",
            "0.01",
            "0.01",
        )]);

        let mut position = models::DefiPosition::new();
        position.position_id = Some("position".to_owned());
        position.investment_ids = Some(vec!["investment".to_owned()]);
        position.position_value = Some("0.71".to_owned());
        position.token_list = Some(Box::new(tokens));
        position.position_detail = Some(Some(Box::new(detail)));

        let mut collection = models::DefiPositionCollection::new();
        collection.position_list = Some(vec![position, models::DefiPosition::new()]);
        let mut pool_detail = models::DefiPoolDetail::new();
        pool_detail.fee_tier = Some("2500".to_owned());
        pool_detail.tick = Some("-58900".to_owned());
        let mut pool = models::DefiPoolPosition::new();
        pool.pool_ca = Some("46A15B0b27311cedF172AB29E4f4766fbE7F4364".to_owned());
        pool.pool_type = Some("Liquidity Pool".to_owned());
        pool.pool_detail = Some(Some(Box::new(pool_detail)));
        pool.position_collection_list = Some(vec![collection]);
        let mut protocol = models::DefiProtocolPosition::new();
        protocol.defi_protocol_id = Some("pancakeswap4".to_owned());
        protocol.protocol_name = Some("PancakeSwap".to_owned());
        protocol.pool_list = Some(vec![pool]);
        let mut address = models::DefiAddressPosition::new();
        address.protocol_list = Some(vec![protocol]);
        let mut positions = models::DefiPositions::new();
        positions.address_list = Some(vec![address]);

        DefiPositionSnapshot {
            positions,
            observed_at_ms: 1_000,
        }
    }

    fn token(contract: &str, symbol: &str, amount: &str, value: &str) -> models::DefiPositionToken {
        let mut token = models::DefiPositionToken::new();
        token.token_address = Some(format!("0x{contract}"));
        token.token_symbol = Some(symbol.to_owned());
        token.token_amount = Some(amount.to_owned());
        token.token_value = Some(value.to_owned());
        token
    }
}
