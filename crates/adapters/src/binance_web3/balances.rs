use std::collections::HashMap;

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

use super::{
    client::BinanceWeb3Client,
    provider::{business_succeeded, parse, required_non_empty},
};

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

        // The provider caps a request at a score of tokens, so a full catalogue
        // needs several; they describe the same instant and are read as one.
        let batches = assets
            .chunks(MAX_TOKENS_PER_REQUEST)
            .map(|batch| self.read_balance_batch(wallet, batch));
        let reads = futures_util::future::try_join_all(batches).await?;
        let observed_at_ms = reads
            .iter()
            .map(|read| read.observed_at_ms)
            .min()
            .ok_or(BalanceReaderError::InvalidData)?;

        Ok(BstockBalanceSnapshot {
            observed_at_ms,
            balances: reads.into_iter().flat_map(|read| read.balances).collect(),
        })
    }
}

struct BalanceBatch {
    observed_at_ms: i64,
    balances: Vec<BstockBalance>,
}

impl BinanceWeb3Client {
    async fn read_balance_batch(
        &self,
        wallet: WalletAddress,
        batch: &[BstockId],
    ) -> Result<BalanceBatch, BalanceReaderError> {
        let response = get_token_balances_by_address(
            &self.configuration,
            GetTokenBalancesByAddressParams {
                get_token_balances_by_address_request: balance_request(wallet, batch),
            },
        )
        .await
        .map_err(|_| BalanceReaderError::Unavailable)?;

        if !business_succeeded(response.code, response.success) {
            return Err(BalanceReaderError::Unavailable);
        }
        let observed_at_ms = response.timestamp.ok_or(BalanceReaderError::InvalidData)?;
        let mut returned = HashMap::with_capacity(batch.len());
        for value in response
            .data
            .unwrap_or_default()
            .into_iter()
            .flat_map(|group| group.token_assets.unwrap_or_default())
        {
            let balance = BstockBalance::try_from(ProviderBalance(value))?;
            returned.insert(balance.id, balance);
        }

        Ok(BalanceBatch {
            observed_at_ms,
            balances: batch
                .iter()
                .map(|id| {
                    returned.remove(id).unwrap_or(BstockBalance {
                        id: *id,
                        raw_amount: U256::ZERO,
                        amount: Decimal::ZERO,
                    })
                })
                .collect(),
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

struct ProviderBalance(TokenBalance);

impl TryFrom<ProviderBalance> for BstockBalance {
    type Error = BalanceReaderError;

    fn try_from(value: ProviderBalance) -> Result<Self, Self::Error> {
        let token = value.0;
        let contract = parse::<Address>(&required_non_empty(token.token_contract_address)?)?;
        let raw_amount = parse(&required_non_empty(token.raw_balance)?)?;
        let amount = parse::<Decimal>(&required_non_empty(token.balance)?)?;

        Ok(Self {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract,
            },
            raw_amount,
            amount,
        })
    }
}
