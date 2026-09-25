use std::collections::{HashMap, HashSet};

use alloy_primitives::U256;
use async_trait::async_trait;
use binance_web3_rwa::{
    apis::wallet_api::{GetTokenBalancesByAddressParams, get_token_balances_by_address},
    models::{
        GetTokenBalancesByAddressRequest, TokenBalance, TokenContractIdentity,
        get_token_balances_by_address_request::ExcludeRiskToken,
    },
};
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_market::{Address, BNB_CHAIN_ID, BstockId},
    wallet_portfolio::{
        BalanceReaderError, BstockBalance, BstockBalanceReader, BstockBalanceSnapshot,
        WalletAddress,
    },
};

use super::client::{BinanceWeb3Client, is_business_success};

const MAX_TOKENS_PER_REQUEST: usize = 20;

#[async_trait]
impl BstockBalanceReader for BinanceWeb3Client {
    async fn read_balances(
        &self,
        wallet: WalletAddress,
        assets: &[BstockId],
    ) -> Result<BstockBalanceSnapshot, BalanceReaderError> {
        if assets.is_empty() {
            return Ok(BstockBalanceSnapshot {
                observed_at_ms: chrono::Utc::now().timestamp_millis(),
                balances: Vec::new(),
            });
        }

        let mut observed_at_ms = None;
        let mut balances = Vec::with_capacity(assets.len());
        for batch in assets.chunks(MAX_TOKENS_PER_REQUEST) {
            let response = get_token_balances_by_address(
                &self.configuration,
                GetTokenBalancesByAddressParams {
                    get_token_balances_by_address_request: balance_request(wallet, batch),
                },
            )
            .await
            .map_err(|_| BalanceReaderError::Unavailable)?;

            if !is_business_success(response.code, response.success) {
                return Err(BalanceReaderError::Unavailable);
            }
            let timestamp = response
                .timestamp
                .filter(|timestamp| *timestamp > 0)
                .ok_or(BalanceReaderError::InvalidData)?;
            observed_at_ms =
                Some(observed_at_ms.map_or(timestamp, |current: i64| current.min(timestamp)));

            let requested = batch.iter().copied().collect::<HashSet<_>>();
            let mut returned = HashMap::with_capacity(batch.len());
            for value in response
                .data
                .ok_or(BalanceReaderError::InvalidData)?
                .into_iter()
                .flat_map(|group| group.token_assets.unwrap_or_default())
            {
                let balance = BstockBalance::try_from(ProviderBalance { value, wallet })?;
                if !requested.contains(&balance.id)
                    || returned.insert(balance.id, balance).is_some()
                {
                    return Err(BalanceReaderError::InvalidData);
                }
            }

            balances.extend(batch.iter().map(|id| {
                returned.remove(id).unwrap_or(BstockBalance {
                    id: *id,
                    raw_amount: U256::ZERO,
                    amount: Decimal::ZERO,
                })
            }));
        }

        Ok(BstockBalanceSnapshot {
            observed_at_ms: observed_at_ms.ok_or(BalanceReaderError::InvalidData)?,
            balances,
        })
    }
}

fn balance_request(wallet: WalletAddress, assets: &[BstockId]) -> GetTokenBalancesByAddressRequest {
    let token_contract_addresses = assets
        .iter()
        .map(|id| TokenContractIdentity::new(id.chain_id.to_string(), id.contract.to_string()))
        .collect();
    let mut request = GetTokenBalancesByAddressRequest::new(
        wallet.as_address().to_string(),
        token_contract_addresses,
    );
    request.exclude_risk_token = Some(ExcludeRiskToken::Variant1);
    request
}

struct ProviderBalance {
    value: TokenBalance,
    wallet: WalletAddress,
}

impl TryFrom<ProviderBalance> for BstockBalance {
    type Error = BalanceReaderError;

    fn try_from(value: ProviderBalance) -> Result<Self, Self::Error> {
        let token = value.value;
        let chain_id = required(token.binance_chain_id)?
            .parse::<u64>()
            .map_err(|_| BalanceReaderError::InvalidData)?;
        if chain_id != BNB_CHAIN_ID {
            return Err(BalanceReaderError::InvalidData);
        }
        let contract = required(token.token_contract_address)?
            .parse::<Address>()
            .map_err(|_| BalanceReaderError::InvalidData)?;
        let response_wallet = required(token.address)?
            .parse::<Address>()
            .map_err(|_| BalanceReaderError::InvalidData)?;
        if response_wallet != value.wallet.as_address() {
            return Err(BalanceReaderError::InvalidData);
        }
        let raw_amount = required(token.raw_balance)?
            .parse()
            .map_err(|_| BalanceReaderError::InvalidData)?;
        let amount = required(token.balance)?
            .parse::<Decimal>()
            .map_err(|_| BalanceReaderError::InvalidData)?;
        if amount < Decimal::ZERO {
            return Err(BalanceReaderError::InvalidData);
        }

        Ok(Self {
            id: BstockId { chain_id, contract },
            raw_amount,
            amount,
        })
    }
}

fn required(value: Option<String>) -> Result<String, BalanceReaderError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or(BalanceReaderError::InvalidData)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_balance_rejects_a_response_for_another_wallet() {
        let wallet = WalletAddress::try_from(Address::with_last_byte(1))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));
        let mut value = TokenBalance::new();
        value.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        value.token_contract_address = Some(Address::with_last_byte(2).to_string());
        value.address = Some(Address::with_last_byte(3).to_string());
        value.raw_balance = Some("1".to_owned());
        value.balance = Some("0.000000000000000001".to_owned());

        let result = BstockBalance::try_from(ProviderBalance { value, wallet });

        assert_eq!(result, Err(BalanceReaderError::InvalidData));
    }
}
