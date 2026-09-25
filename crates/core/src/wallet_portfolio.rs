use std::{collections::HashMap, sync::Arc};

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

use crate::bstock_market::{BNB_CHAIN_ID, BstockCatalog, BstockId, BstockSummary, CatalogError};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WalletAddress(Address);

impl WalletAddress {
    #[must_use]
    pub const fn as_address(self) -> Address {
        self.0
    }
}

impl TryFrom<Address> for WalletAddress {
    type Error = WalletAddressError;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        if value == Address::ZERO {
            return Err(WalletAddressError::Zero);
        }
        Ok(Self(value))
    }
}

impl Serialize for WalletAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for WalletAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let address = String::deserialize(deserializer)?
            .parse::<Address>()
            .map_err(de::Error::custom)?;
        Self::try_from(address).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum WalletAddressError {
    #[error("wallet address cannot be the zero address")]
    Zero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BstockBalance {
    pub id: BstockId,
    pub raw_amount: U256,
    pub amount: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BstockBalanceSnapshot {
    pub observed_at_ms: i64,
    pub balances: Vec<BstockBalance>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BstockPosition {
    pub bstock: BstockSummary,
    #[serde(serialize_with = "serialize_u256")]
    pub raw_amount: U256,
    pub token_amount: Decimal,
    pub share_exposure: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPortfolio {
    pub wallet: WalletAddress,
    pub chain_id: u64,
    pub observed_at_ms: i64,
    pub positions: Vec<BstockPosition>,
}

fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(value)
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BalanceReaderError {
    #[error("the balance provider is temporarily unavailable")]
    Unavailable,
    #[error("the balance provider returned invalid data")]
    InvalidData,
}

#[async_trait]
pub trait BstockBalanceReader: Send + Sync {
    async fn read_balances(
        &self,
        wallet: WalletAddress,
        assets: &[BstockId],
    ) -> Result<BstockBalanceSnapshot, BalanceReaderError>;
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WalletPortfolioError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    BalanceReader(#[from] BalanceReaderError),
    #[error("the balance reader returned an inconsistent snapshot")]
    InvalidSnapshot,
}

#[derive(Clone)]
pub struct WalletPortfolioService {
    catalog: Arc<dyn BstockCatalog>,
    balance_reader: Arc<dyn BstockBalanceReader>,
}

impl WalletPortfolioService {
    pub fn new(
        catalog: Arc<dyn BstockCatalog>,
        balance_reader: Arc<dyn BstockBalanceReader>,
    ) -> Self {
        Self {
            catalog,
            balance_reader,
        }
    }

    /// Returns the wallet's bStock positions from one provider observation.
    ///
    /// # Errors
    ///
    /// Returns [`WalletPortfolioError`] when catalog or balance discovery fails or exposure
    /// arithmetic overflows.
    pub async fn portfolio(
        &self,
        wallet: WalletAddress,
    ) -> Result<WalletPortfolio, WalletPortfolioError> {
        let assets = self.catalog.list_bstocks().await?;
        let ids = assets.iter().map(|asset| asset.id).collect::<Vec<_>>();
        let snapshot = self.balance_reader.read_balances(wallet, &ids).await?;
        let positions = positions_from_snapshot(assets, snapshot.balances)?;

        Ok(WalletPortfolio {
            wallet,
            chain_id: BNB_CHAIN_ID,
            observed_at_ms: snapshot.observed_at_ms,
            positions,
        })
    }
}

fn positions_from_snapshot(
    assets: Vec<BstockSummary>,
    balances: Vec<BstockBalance>,
) -> Result<Vec<BstockPosition>, WalletPortfolioError> {
    let mut balances_by_id = balances
        .into_iter()
        .map(|balance| (balance.id, balance))
        .collect::<HashMap<_, _>>();

    let mut positions = Vec::new();
    for asset in assets {
        let balance = balances_by_id.remove(&asset.id).unwrap_or(BstockBalance {
            id: asset.id,
            raw_amount: U256::ZERO,
            amount: Decimal::ZERO,
        });
        if balance.raw_amount == U256::ZERO {
            continue;
        }

        let share_exposure = balance
            .amount
            .checked_mul(asset.token_to_share_ratio)
            .ok_or(WalletPortfolioError::InvalidSnapshot)?;

        positions.push(BstockPosition {
            bstock: asset,
            raw_amount: balance.raw_amount,
            token_amount: balance.amount,
            share_exposure,
        });
    }

    positions.sort_by(|left, right| {
        left.bstock
            .underlying_ticker
            .cmp(&right.bstock.underlying_ticker)
    });
    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bstock_market::{AssetKind, BstockDetails};

    struct FixedCatalog(Vec<BstockSummary>);

    #[async_trait]
    impl BstockCatalog for FixedCatalog {
        async fn list_bstocks(&self) -> Result<Vec<BstockSummary>, CatalogError> {
            Ok(self.0.clone())
        }

        async fn find_bstock(&self, _id: BstockId) -> Result<Option<BstockDetails>, CatalogError> {
            Ok(None)
        }
    }

    struct FixedBalanceReader(BstockBalanceSnapshot);

    #[async_trait]
    impl BstockBalanceReader for FixedBalanceReader {
        async fn read_balances(
            &self,
            _wallet: WalletAddress,
            _assets: &[BstockId],
        ) -> Result<BstockBalanceSnapshot, BalanceReaderError> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn portfolio_calculates_share_exposure_and_excludes_zero_balances() {
        let mut nvda = summary(1, "NVDA");
        nvda.token_to_share_ratio = Decimal::new(15, 1);
        let aapl = summary(2, "AAPL");
        let reader = FixedBalanceReader(BstockBalanceSnapshot {
            observed_at_ms: 42,
            balances: vec![
                balance(nvda.id, 2, Decimal::from(2)),
                balance(aapl.id, 0, Decimal::ZERO),
            ],
        });
        let service = WalletPortfolioService::new(
            Arc::new(FixedCatalog(vec![nvda.clone(), aapl])),
            Arc::new(reader),
        );
        let wallet = WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));

        let portfolio = service
            .portfolio(wallet)
            .await
            .unwrap_or_else(|error| panic!("portfolio failed: {error}"));

        assert_eq!(portfolio.observed_at_ms, 42);
        assert_eq!(portfolio.positions.len(), 1);
        assert_eq!(portfolio.positions[0].bstock, nvda);
        assert_eq!(portfolio.positions[0].token_amount, Decimal::from(2));
        assert_eq!(portfolio.positions[0].share_exposure, Decimal::from(3));
    }

    #[tokio::test]
    async fn portfolio_rejects_share_exposure_overflow() {
        let mut asset = summary(1, "NVDA");
        asset.token_to_share_ratio = Decimal::MAX;
        let service = WalletPortfolioService::new(
            Arc::new(FixedCatalog(vec![asset.clone()])),
            Arc::new(FixedBalanceReader(BstockBalanceSnapshot {
                observed_at_ms: 42,
                balances: vec![balance(asset.id, 2, Decimal::from(2))],
            })),
        );
        let wallet = WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"));
        let error = service
            .portfolio(wallet)
            .await
            .expect_err("an unrepresentable share exposure must be rejected");

        assert_eq!(error, WalletPortfolioError::InvalidSnapshot);
    }

    fn balance(id: BstockId, raw_amount: u128, amount: Decimal) -> BstockBalance {
        BstockBalance {
            id,
            raw_amount: U256::from(raw_amount),
            amount,
        }
    }

    fn summary(last_address_byte: u8, ticker: &str) -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: Address::with_last_byte(last_address_byte),
            },
            token_name: format!("{ticker} bStock"),
            token_symbol: format!("{ticker}B"),
            token_logo_url: None,
            token_decimals: 18,
            underlying_ticker: ticker.to_owned(),
            underlying_name: ticker.to_owned(),
            asset_kind: AssetKind::Stock,
            token_to_share_ratio: Decimal::ONE,
            token_price_usd: None,
            reference_price_usd: None,
            volume_24h_usd: None,
            market_cap_usd: None,
            market_status: None,
            tags: Vec::new(),
        }
    }
}
