use std::str::FromStr;

use alloy_primitives::{Address, B256, U256};
use chrono::DateTime;
use glider_api::models::{
    CreateEnrollData, CreateEnrollSignatureData, DiscoveryStrategyItem, PortfolioAsset,
    PortfolioDetail, PortfolioListItem, PortfolioPositionsData, PortfolioSchedule, SmartAccount,
    StrategyAllocationAsset,
    portfolio_position_warning::Kind as PositionWarningKind,
    portfolio_schedule::{Frequency, Status},
};
use rust_decimal::Decimal;
use stockfront_core::{
    baskets::BasketKind,
    bstock_market::BNB_CHAIN_ID,
    managed_portfolios::{
        ManagedPortfolioAccount, ManagedPortfolioAllocation, ManagedPortfolioAssetPosition,
        ManagedPortfolioAutomationStatus, ManagedPortfolioEnrollment,
        ManagedPortfolioEnrollmentAuthorization, ManagedPortfolioFrequency,
        ManagedPortfolioPerformance, ManagedPortfolioPositions, ManagedPortfolioProviderError,
        ManagedPortfolioSchedule, ManagedPortfolioSummary, ManagedPortfolioTemplate,
        ManagedPortfolioTemplateMetrics, ManagedPortfolioWarning, ManagedPortfolioWarningKind,
        PreparedManagedPortfolioEnrollment,
    },
};

const BNB_ERC20_PREFIX: &str = "eip155:56/erc20:";
const BNB_ACCOUNT_PREFIX: &str = "eip155:56:";

pub(super) struct BnbManagedPortfolioTemplate(ManagedPortfolioTemplate);

struct BnbManagedPortfolioAllocation(ManagedPortfolioAllocation);

pub(super) struct BnbPreparedManagedPortfolioEnrollment(PreparedManagedPortfolioEnrollment);

pub(super) struct BnbManagedPortfolioEnrollment(ManagedPortfolioEnrollment);

pub(super) struct BnbManagedPortfolioSummary(ManagedPortfolioSummary);

pub(super) struct BnbManagedPortfolioPositions(ManagedPortfolioPositions);

pub(super) struct BnbManagedPortfolioDepositAccount(ManagedPortfolioAccount);

impl From<BnbManagedPortfolioTemplate> for ManagedPortfolioTemplate {
    fn from(value: BnbManagedPortfolioTemplate) -> Self {
        value.0
    }
}

impl From<BnbManagedPortfolioAllocation> for ManagedPortfolioAllocation {
    fn from(value: BnbManagedPortfolioAllocation) -> Self {
        value.0
    }
}

impl From<BnbPreparedManagedPortfolioEnrollment> for PreparedManagedPortfolioEnrollment {
    fn from(value: BnbPreparedManagedPortfolioEnrollment) -> Self {
        value.0
    }
}

impl From<BnbManagedPortfolioEnrollment> for ManagedPortfolioEnrollment {
    fn from(value: BnbManagedPortfolioEnrollment) -> Self {
        value.0
    }
}

impl From<BnbManagedPortfolioSummary> for ManagedPortfolioSummary {
    fn from(value: BnbManagedPortfolioSummary) -> Self {
        value.0
    }
}

impl From<BnbManagedPortfolioPositions> for ManagedPortfolioPositions {
    fn from(value: BnbManagedPortfolioPositions) -> Self {
        value.0
    }
}

impl BnbManagedPortfolioDepositAccount {
    pub(super) fn into_inner(self) -> ManagedPortfolioAccount {
        self.0
    }
}

impl TryFrom<DiscoveryStrategyItem> for BnbManagedPortfolioTemplate {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: DiscoveryStrategyItem) -> Result<Self, Self::Error> {
        let name = non_empty(&value.name).ok_or(ManagedPortfolioProviderError::InvalidData)?;
        let allocation = value
            .allocation
            .assets
            .into_iter()
            .map(BnbManagedPortfolioAllocation::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let allocation = allocation
            .into_iter()
            .map(ManagedPortfolioAllocation::from)
            .collect();
        let metrics = *value.metrics;
        let performance = metrics
            .performance
            .summary
            .windows
            .into_iter()
            .filter_map(|window| {
                Decimal::from_str(&window.percent_change)
                    .ok()
                    .map(|percent_change| ManagedPortfolioPerformance {
                        window: window.window,
                        percent_change,
                        since: window.since,
                    })
            })
            .collect();

        Ok(Self(ManagedPortfolioTemplate {
            id: value.strategy_id,
            kind: BasketKind::ManagedPortfolio,
            provider_id: "glider".to_owned(),
            provider_name: "Glider".to_owned(),
            name,
            description: value.description.as_deref().and_then(non_empty),
            advertised_max_apy_percent: value.max_apy.and_then(|apy| Decimal::from_str(&apy).ok()),
            allocation,
            metrics: ManagedPortfolioTemplateMetrics {
                tvl_usd: metrics.tvl_usd.and_then(|tvl| Decimal::from_str(&tvl).ok()),
                portfolio_count: metrics
                    .portfolio_count
                    .and_then(|count| u64::try_from(count).ok()),
                performance,
            },
        }))
    }
}

impl TryFrom<StrategyAllocationAsset> for BnbManagedPortfolioAllocation {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: StrategyAllocationAsset) -> Result<Self, Self::Error> {
        let contract = value
            .asset_id
            .strip_prefix(BNB_ERC20_PREFIX)
            .ok_or(ManagedPortfolioProviderError::InvalidData)?
            .parse::<Address>()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        let weight_percent = Decimal::from_str(&value.weight)
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;

        Ok(Self(ManagedPortfolioAllocation {
            chain_id: BNB_CHAIN_ID,
            contract,
            weight_percent,
        }))
    }
}

impl TryFrom<CreateEnrollSignatureData> for BnbPreparedManagedPortfolioEnrollment {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: CreateEnrollSignatureData) -> Result<Self, Self::Error> {
        let personal_sign_message = B256::from_str(&value.message.raw)
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;

        Ok(Self(PreparedManagedPortfolioEnrollment {
            authorization: ManagedPortfolioEnrollmentAuthorization {
                flow_id: value.flow_id,
                account_index: value.account_index,
                agent_account_id: value.agent_account_id,
            },
            personal_sign_message,
        }))
    }
}

impl TryFrom<CreateEnrollData> for BnbManagedPortfolioEnrollment {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: CreateEnrollData) -> Result<Self, Self::Error> {
        let [smart_account] = value
            .smart_accounts
            .try_into()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        let address = smart_account
            .account_id
            .strip_prefix(BNB_ACCOUNT_PREFIX)
            .ok_or(ManagedPortfolioProviderError::InvalidData)?
            .parse::<Address>()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?;

        Ok(Self(ManagedPortfolioEnrollment {
            portfolio_id: value.portfolio_id,
            template_id: value.strategy_id,
            name: value.portfolio_name,
            smart_account: ManagedPortfolioAccount {
                chain_id: BNB_CHAIN_ID,
                address,
            },
        }))
    }
}

impl TryFrom<PortfolioListItem> for BnbManagedPortfolioSummary {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: PortfolioListItem) -> Result<Self, Self::Error> {
        let template_name =
            non_empty(&value.strategy_name).ok_or(ManagedPortfolioProviderError::InvalidData)?;
        let smart_account = value
            .smart_accounts
            .iter()
            .find_map(bnb_account)
            .ok_or(ManagedPortfolioProviderError::InvalidData)?;
        let schedule = value.schedule.as_deref().map(map_schedule).transpose()?;

        Ok(Self(ManagedPortfolioSummary {
            portfolio_id: value.portfolio_id,
            name: value.portfolio_name.as_deref().and_then(non_empty),
            template_id: value.strategy_id,
            template_name,
            smart_account,
            schedule,
        }))
    }
}

impl TryFrom<PortfolioPositionsData> for BnbManagedPortfolioPositions {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: PortfolioPositionsData) -> Result<Self, Self::Error> {
        let smart_account = value
            .smart_accounts
            .iter()
            .find_map(bnb_account)
            .ok_or(ManagedPortfolioProviderError::InvalidData)?;
        let assets = value
            .assets
            .into_iter()
            .map(map_bnb_asset)
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, _>>()?;
        let total_value_usd = assets.iter().map(|asset| asset.value_usd).sum::<Decimal>();
        let warnings = value
            .warnings
            .into_iter()
            .map(|warning| ManagedPortfolioWarning {
                kind: match warning.kind {
                    PositionWarningKind::MissingSmartAccount => {
                        ManagedPortfolioWarningKind::MissingSmartAccount
                    }
                    PositionWarningKind::RpcError => ManagedPortfolioWarningKind::RpcError,
                    PositionWarningKind::MissingPrice => ManagedPortfolioWarningKind::MissingPrice,
                    PositionWarningKind::BlocklistedAsset => {
                        ManagedPortfolioWarningKind::BlocklistedAsset
                    }
                    PositionWarningKind::MissingAsset => ManagedPortfolioWarningKind::MissingAsset,
                    PositionWarningKind::InTransitReadFailed => {
                        ManagedPortfolioWarningKind::InTransitReadFailed
                    }
                },
                message: warning.message,
            })
            .collect();

        Ok(Self(ManagedPortfolioPositions {
            portfolio_id: value.portfolio_id,
            smart_account,
            total_value_usd,
            assets,
            warnings,
            observed_at_ms: parse_timestamp(&value.fetched_at)?,
        }))
    }
}

impl TryFrom<PortfolioDetail> for BnbManagedPortfolioDepositAccount {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: PortfolioDetail) -> Result<Self, Self::Error> {
        value
            .smart_accounts
            .iter()
            .find_map(bnb_deposit_account)
            .map(Self)
            .ok_or(ManagedPortfolioProviderError::InvalidData)
    }
}

fn map_schedule(
    schedule: &PortfolioSchedule,
) -> Result<ManagedPortfolioSchedule, ManagedPortfolioProviderError> {
    Ok(ManagedPortfolioSchedule {
        status: match schedule.status {
            Status::Active => ManagedPortfolioAutomationStatus::Active,
            Status::Paused => ManagedPortfolioAutomationStatus::Paused,
        },
        frequency: schedule.frequency.map(|frequency| match frequency {
            Frequency::Hourly => ManagedPortfolioFrequency::Hourly,
            Frequency::Daily => ManagedPortfolioFrequency::Daily,
            Frequency::Weekly => ManagedPortfolioFrequency::Weekly,
            Frequency::Monthly => ManagedPortfolioFrequency::Monthly,
        }),
        interval_ms: schedule
            .interval_ms
            .map(u64::try_from)
            .transpose()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?,
        next_rebalance_at_ms: schedule
            .next_due_at
            .as_deref()
            .map(parse_timestamp)
            .transpose()?,
        last_rebalance_at_ms: schedule
            .last_rebalance_at
            .as_deref()
            .map(parse_timestamp)
            .transpose()?,
    })
}

fn map_bnb_asset(
    asset: PortfolioAsset,
) -> Result<Option<ManagedPortfolioAssetPosition>, ManagedPortfolioProviderError> {
    if !asset.smart_account_id.starts_with(BNB_ACCOUNT_PREFIX) {
        return Ok(None);
    }
    let Some(contract) = asset.asset_id.strip_prefix(BNB_ERC20_PREFIX) else {
        return Ok(None);
    };

    Ok(Some(ManagedPortfolioAssetPosition {
        contract: contract
            .parse()
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?,
        symbol: asset.symbol,
        decimals: u8::try_from(asset.decimals)
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?,
        raw_amount: U256::from_str(&asset.balance_raw)
            .map_err(|_| ManagedPortfolioProviderError::InvalidData)?,
        amount: parse_decimal(&asset.balance)?,
        price_usd: parse_decimal(&asset.price_usd)?,
        value_usd: parse_decimal(&asset.value_usd)?,
    }))
}

fn bnb_account(account: &SmartAccount) -> Option<ManagedPortfolioAccount> {
    parse_bnb_account(&account.account_id)
}

fn bnb_deposit_account(account: &SmartAccount) -> Option<ManagedPortfolioAccount> {
    account
        .deposit_account_id
        .as_deref()
        .and_then(parse_bnb_account)
        .or_else(|| parse_bnb_account(&account.account_id))
}

fn parse_bnb_account(account_id: &str) -> Option<ManagedPortfolioAccount> {
    let address = account_id.strip_prefix(BNB_ACCOUNT_PREFIX)?.parse().ok()?;
    Some(ManagedPortfolioAccount {
        chain_id: BNB_CHAIN_ID,
        address,
    })
}

fn parse_decimal(value: &str) -> Result<Decimal, ManagedPortfolioProviderError> {
    Decimal::from_str(value).map_err(|_| ManagedPortfolioProviderError::InvalidData)
}

pub(super) fn parse_timestamp(value: &str) -> Result<i64, ManagedPortfolioProviderError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.timestamp_millis())
        .map_err(|_| ManagedPortfolioProviderError::InvalidData)
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bnb_strategy_and_rejects_other_chains() {
        let strategy = serde_json::from_value::<DiscoveryStrategyItem>(serde_json::json!({
            "strategyId": "01K6D5JCKM1N7Z8D2YQ0Q3F1AB",
            "name": "Technology Leaders",
            "description": "  BNB stocks  ",
            "maxApy": "10.25",
            "allocation": {
                "assets": [{
                    "assetId": "eip155:56/erc20:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
                    "weight": "100"
                }]
            },
            "createdAt": "2026-09-30T00:00:00.000Z",
            "metrics": {
                "tvlUsd": "1200.50",
                "portfolioCount": 4,
                "performance": {
                    "summary": {
                        "windows": [{
                            "window": "1m",
                            "percentChange": "2.75",
                            "since": "2026-08-30"
                        }]
                    }
                }
            },
            "canMirror": true
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        let template = ManagedPortfolioTemplate::from(
            BnbManagedPortfolioTemplate::try_from(strategy)
                .unwrap_or_else(|error| panic!("valid BNB strategy rejected: {error}")),
        );

        assert_eq!(template.kind, BasketKind::ManagedPortfolio);
        assert_eq!(template.description.as_deref(), Some("BNB stocks"));
        assert_eq!(
            template.advertised_max_apy_percent,
            Some(Decimal::new(1_025, 2))
        );
        assert_eq!(template.allocation.len(), 1);
        assert_eq!(template.allocation[0].chain_id, BNB_CHAIN_ID);
        assert_eq!(template.metrics.tvl_usd, Some(Decimal::new(120_050, 2)));
        assert_eq!(template.metrics.portfolio_count, Some(4));
        assert_eq!(template.metrics.performance[0].window, "1m");

        let base_strategy = serde_json::from_value::<DiscoveryStrategyItem>(serde_json::json!({
            "strategyId": "01K6D5JCKM1N7Z8D2YQ0Q3F1AC",
            "name": "Base strategy",
            "description": null,
            "allocation": {
                "assets": [{
                    "assetId": "eip155:8453/erc20:0x0000000000000000000000000000000000000001",
                    "weight": "100"
                }]
            },
            "createdAt": null,
            "metrics": {
                "tvlUsd": null,
                "portfolioCount": null,
                "performance": { "summary": { "windows": [] } }
            },
            "canMirror": true
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        assert!(BnbManagedPortfolioTemplate::try_from(base_strategy).is_err());
    }

    #[test]
    fn maps_bnb_enrollment_authorization_and_result() {
        let prepared = serde_json::from_value::<CreateEnrollSignatureData>(serde_json::json!({
            "message": {
                "kind": "ecdsa",
                "raw": "0x595dee26258ccd236c246fcb00aaec8698d33ca91be55f38d451d67baa99e564"
            },
            "agentAccountId": "eip155:0:0xa53835d3eb7a8a6666c4771c29bc4db546137b56",
            "accountIndex": "70",
            "accountType": "ECDSA",
            "flowId": "01M3SFGSCT4BND50BVBKBV4YTZ"
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));
        let prepared = PreparedManagedPortfolioEnrollment::from(
            BnbPreparedManagedPortfolioEnrollment::try_from(prepared)
                .unwrap_or_else(|error| panic!("valid enrollment authorization rejected: {error}")),
        );

        assert_eq!(prepared.authorization.account_index, "70");
        assert_eq!(
            prepared.personal_sign_message,
            B256::from_str("0x595dee26258ccd236c246fcb00aaec8698d33ca91be55f38d451d67baa99e564")
                .unwrap_or_else(|error| panic!("valid hash rejected: {error}"))
        );

        let enrollment = serde_json::from_value::<CreateEnrollData>(serde_json::json!({
            "portfolioId": "portfolio-1",
            "portfolioName": "Technology Portfolio",
            "strategyId": "strategy-1",
            "smartAccounts": [{
                "accountId": "eip155:56:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436"
            }]
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));
        let enrollment = ManagedPortfolioEnrollment::from(
            BnbManagedPortfolioEnrollment::try_from(enrollment)
                .unwrap_or_else(|error| panic!("valid BNB enrollment rejected: {error}")),
        );

        assert_eq!(enrollment.portfolio_id, "portfolio-1");
        assert_eq!(enrollment.template_id, "strategy-1");
        assert_eq!(enrollment.smart_account.chain_id, BNB_CHAIN_ID);
        assert_eq!(
            enrollment.smart_account.address,
            "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436"
                .parse::<Address>()
                .unwrap_or_else(|error| panic!("valid address rejected: {error}"))
        );
    }

    #[test]
    fn maps_bnb_portfolio_summary_and_schedule() {
        let portfolio = serde_json::from_value::<PortfolioListItem>(serde_json::json!({
            "portfolioId": "portfolio-1",
            "portfolioName": "  Technology Portfolio  ",
            "ownerAccountId": "eip155:0:0x4b393bEc5DeE11799F698440AB19c0FBfe15d74c",
            "strategyId": "strategy-1",
            "strategyName": "Technology Leaders",
            "strategyDescription": null,
            "smartAccounts": [{
                "accountId": "eip155:56:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436"
            }],
            "schedule": {
                "status": "active",
                "frequency": "monthly",
                "intervalMs": 2_592_000_000_i64,
                "nextDueAt": "2026-10-01T12:00:00.000Z",
                "lastRebalanceAt": null
            },
            "createdAt": "2026-09-30T12:00:00.000Z"
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        let summary = ManagedPortfolioSummary::from(
            BnbManagedPortfolioSummary::try_from(portfolio)
                .unwrap_or_else(|error| panic!("valid BNB portfolio rejected: {error}")),
        );

        assert_eq!(summary.name.as_deref(), Some("Technology Portfolio"));
        assert_eq!(summary.template_id, "strategy-1");
        assert_eq!(summary.smart_account.chain_id, BNB_CHAIN_ID);
        let schedule = summary.schedule.expect("test portfolio has a schedule");
        assert_eq!(schedule.status, ManagedPortfolioAutomationStatus::Active);
        assert_eq!(schedule.frequency, Some(ManagedPortfolioFrequency::Monthly));
        assert_eq!(schedule.interval_ms, Some(2_592_000_000));
        assert_eq!(
            schedule.next_rebalance_at_ms,
            Some(
                DateTime::parse_from_rfc3339("2026-10-01T12:00:00.000Z")
                    .expect("test timestamp is valid")
                    .timestamp_millis()
            )
        );
    }

    #[test]
    fn resolves_bnb_portfolio_deposit_account() {
        let portfolio = serde_json::from_value::<PortfolioDetail>(serde_json::json!({
            "smartAccounts": [
                { "accountId": "eip155:8453:0x2222222222222222222222222222222222222222" },
                { "accountId": "eip155:56:0x1111111111111111111111111111111111111111" }
            ]
        }))
        .unwrap_or_else(|error| panic!("valid portfolio detail rejected: {error}"));

        let account = BnbManagedPortfolioDepositAccount::try_from(portfolio)
            .unwrap_or_else(|error| panic!("BNB deposit account rejected: {error}"))
            .into_inner();

        assert_eq!(account.chain_id, BNB_CHAIN_ID);
        assert_eq!(
            account.address,
            "0x1111111111111111111111111111111111111111"
                .parse::<Address>()
                .unwrap_or_else(|error| panic!("test address rejected: {error}"))
        );
    }

    #[test]
    fn maps_exact_bnb_positions_and_preserves_warnings() {
        let positions = serde_json::from_value::<PortfolioPositionsData>(serde_json::json!({
            "portfolioId": "portfolio-1",
            "totalValueUsd": "223.760000",
            "inTransit": { "totalUsd": "0.000000", "items": [] },
            "fetchedAt": "2026-09-30T12:05:10.000Z",
            "smartAccounts": [{
                "accountId": "eip155:56:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436"
            }],
            "assets": [{
                "assetId": "eip155:56/erc20:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
                "smartAccountId": "eip155:56:0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
                "symbol": "NVDAB",
                "decimals": 18,
                "balance": "1.000000000000000001",
                "balanceRaw": "1000000000000000001",
                "priceUsd": "223.760000",
                "valueUsd": "223.760000"
            }, {
                "assetId": "eip155:8453/erc20:0x0000000000000000000000000000000000000001",
                "smartAccountId": "eip155:8453:0x0000000000000000000000000000000000000002",
                "symbol": "OTHER",
                "decimals": 18,
                "balance": "1",
                "balanceRaw": "1000000000000000000",
                "priceUsd": "1",
                "valueUsd": "1"
            }],
            "warnings": [{
                "kind": "MISSING_PRICE",
                "message": "one upstream price was unavailable"
            }]
        }))
        .unwrap_or_else(|error| panic!("valid generated model rejected: {error}"));

        let positions = ManagedPortfolioPositions::from(
            BnbManagedPortfolioPositions::try_from(positions)
                .unwrap_or_else(|error| panic!("valid BNB positions rejected: {error}")),
        );

        assert_eq!(positions.total_value_usd, Decimal::new(22_376, 2));
        assert_eq!(positions.assets.len(), 1);
        assert_eq!(positions.assets[0].decimals, 18);
        assert_eq!(
            positions.assets[0].raw_amount,
            U256::from(1_000_000_000_000_000_001_u128)
        );
        assert_eq!(
            positions.assets[0].amount,
            Decimal::from_str("1.000000000000000001").expect("test amount is valid")
        );
        assert_eq!(positions.warnings.len(), 1);
        assert_eq!(
            positions.warnings[0].kind,
            ManagedPortfolioWarningKind::MissingPrice
        );
    }
}
