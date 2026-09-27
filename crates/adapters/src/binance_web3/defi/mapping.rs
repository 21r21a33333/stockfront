use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingPreview, BorrowingProviderError, BorrowingTransactionKind,
        BorrowingTransactionState, BorrowingTransactionStatus, PreparedBorrowingAction,
    },
    carry::{
        CarryBalanceChange, CarryDepositPlan, CarryDepositPreview, CarryTransactionKind,
        EarnProviderError,
    },
    prepared_action::{ActionWarning, NetworkFee, PreparedTransaction},
};

use super::super::provider::{
    InvalidProviderData, parse, parse_optional, parse_optional_u256, required,
};

pub(super) struct ProviderPreparedAction(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderPreparedAction> for PreparedBorrowingAction {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderPreparedAction) -> Result<Self, Self::Error> {
        let plan = value.0;
        let transactions = prepared_transactions(plan.data_list, |kind| match kind {
            models::defi_transaction::CallDataType::Approve => {
                Some(BorrowingTransactionKind::Approve)
            }
            models::defi_transaction::CallDataType::Deposit => {
                Some(BorrowingTransactionKind::Deposit)
            }
            _ => None,
        })?;
        let preview = plan
            .preview
            .map(|preview| BorrowingPreview::try_from(ProviderActionPreview(*preview)))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            transactions,
            preview,
        })
    }
}

fn prepared_transactions<K>(
    transactions: Option<Vec<models::DefiTransaction>>,
    kind: impl Fn(models::defi_transaction::CallDataType) -> Option<K>,
) -> Result<Vec<PreparedTransaction<K>>, InvalidProviderData> {
    transactions
        .filter(|transactions| !transactions.is_empty())
        .ok_or(InvalidProviderData)?
        .into_iter()
        .map(|transaction| {
            let transaction_kind =
                kind(required(transaction.call_data_type)?).ok_or(InvalidProviderData)?;
            Ok(PreparedTransaction {
                kind: transaction_kind,
                to: parse(&required(transaction.to.flatten())?)?,
                data: parse(&required(transaction.data)?)?,
                value: parse_optional_u256(transaction.value.flatten().as_deref())?
                    .unwrap_or_default(),
                gas_limit: parse_optional_u256(transaction.gas_limit.flatten().as_deref())?,
                gas_price: parse_optional_u256(transaction.gas_price.flatten().as_deref())?,
                max_priority_fee_per_gas: parse_optional_u256(
                    transaction.max_priority_fee_per_gas.flatten().as_deref(),
                )?,
                max_fee_per_gas: parse_optional_u256(
                    transaction.max_fee_per_gas.flatten().as_deref(),
                )?,
            })
        })
        .collect()
}

struct ProviderActionPreview(models::DefiTransactionPreview);

impl TryFrom<ProviderActionPreview> for BorrowingPreview {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderActionPreview) -> Result<Self, Self::Error> {
        let value = value.0;
        let estimated_network_fee = network_fee(&value)?;
        let (health_factor_before, health_factor_after) = value
            .health_factor
            .as_deref()
            .map(|health_factor| -> Result<_, BorrowingProviderError> {
                let before: Option<Decimal> = parse_optional(
                    health_factor
                        .before
                        .as_ref()
                        .and_then(Option::as_ref)
                        .map(String::as_str),
                )?;
                let after: Option<Decimal> = parse_optional(
                    health_factor
                        .after
                        .as_ref()
                        .and_then(Option::as_ref)
                        .map(String::as_str),
                )?;
                Ok((before, after))
            })
            .transpose()?
            .unwrap_or_default();
        let (health_factor_before, health_factor_after) =
            normalize_health_factors(health_factor_before, health_factor_after);
        let warnings = action_warnings(&value);
        Ok(Self {
            estimated_network_fee,
            health_factor_before,
            health_factor_after,
            warnings,
        })
    }
}

fn network_fee(
    preview: &models::DefiTransactionPreview,
) -> Result<Option<NetworkFee>, InvalidProviderData> {
    preview
        .fee_and_contract
        .as_deref()
        .and_then(|fee| fee.estimated_network_fee.as_deref())
        .map(|fee| {
            Ok(NetworkFee {
                amount: parse(&required(fee.amount.clone().flatten())?)?,
                token_symbol: required(fee.token_symbol.clone())?,
                value_usd: parse_optional(fee.value_usd.clone().flatten().as_deref())?,
            })
        })
        .transpose()
}

fn action_warnings(preview: &models::DefiTransactionPreview) -> Vec<ActionWarning> {
    preview
        .warnings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|warning| {
            Some(ActionWarning {
                code: warning.code.clone()?,
                level: warning.level.clone()?,
                message: warning.message.clone()?,
            })
        })
        .collect()
}

pub(super) struct ProviderCarryDepositPlan(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderCarryDepositPlan> for CarryDepositPlan {
    type Error = EarnProviderError;

    fn try_from(value: ProviderCarryDepositPlan) -> Result<Self, Self::Error> {
        let plan = value.0;
        let transactions = prepared_transactions(plan.data_list, |kind| match kind {
            models::defi_transaction::CallDataType::Approve => Some(CarryTransactionKind::Approve),
            models::defi_transaction::CallDataType::Deposit => Some(CarryTransactionKind::Deposit),
            _ => None,
        })?;
        if !matches!(
            transactions.last().map(|transaction| transaction.kind),
            Some(CarryTransactionKind::Deposit)
        ) {
            return Err(EarnProviderError::InvalidData);
        }
        let preview = required(plan.preview)?;
        let balance_changes = preview
            .balance_change
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|change| {
                Ok(CarryBalanceChange {
                    token: stockfront_core::bstock_borrowing::BorrowingToken {
                        contract: parse(&required(change.token_address.clone())?)?,
                        symbol: required(change.token_symbol.clone())?,
                    },
                    amount: parse(&required(change.amount.clone())?)?,
                    value_usd: parse_optional(change.value_usd.clone().flatten().as_deref())?,
                })
            })
            .collect::<Result<Vec<_>, InvalidProviderData>>()?;
        let interact_with = preview
            .fee_and_contract
            .as_deref()
            .and_then(|fee| fee.interact_with.as_deref())
            .and_then(|contract| contract.address.as_deref())
            .map(parse)
            .transpose()?;
        let preview = CarryDepositPreview {
            balance_changes,
            estimated_network_fee: network_fee(&preview)?,
            interact_with,
            warnings: action_warnings(&preview),
        };

        Ok(Self {
            transactions,
            preview,
        })
    }
}

fn normalize_health_factors(
    before: Option<Decimal>,
    after: Option<Decimal>,
) -> (Option<Decimal>, Option<Decimal>) {
    if before == Some(Decimal::ZERO) && after == Some(Decimal::ZERO) {
        (None, None)
    } else {
        (before, after)
    }
}

pub(super) struct ProviderTransactionDetails(pub(super) Vec<models::TransactionDetail>);

impl TryFrom<ProviderTransactionDetails> for BorrowingTransactionStatus {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderTransactionDetails) -> Result<Self, Self::Error> {
        let details = value.0;
        let state = transaction_state(&details);
        let block_number = details
            .iter()
            .filter_map(|detail| detail.height.as_deref())
            .map(parse)
            .next()
            .transpose()?;
        let executed_at_ms = details
            .iter()
            .filter_map(|detail| detail.tx_time.as_deref())
            .map(parse)
            .next()
            .transpose()?;
        Ok(Self {
            state,
            block_number,
            executed_at_ms,
        })
    }
}

fn transaction_state(details: &[models::TransactionDetail]) -> BorrowingTransactionState {
    let mut saw_pending = false;
    let mut saw_unknown = false;
    for state in details
        .iter()
        .filter_map(|detail| detail.tx_status.as_deref())
    {
        match state.trim().to_ascii_uppercase().as_str() {
            "FAIL" | "FAILED" | "REVERTED" => return BorrowingTransactionState::Failed,
            "PENDING" => saw_pending = true,
            "SUCCESS" | "SUCCEEDED" => {}
            _ => saw_unknown = true,
        }
    }
    if saw_pending {
        BorrowingTransactionState::Pending
    } else if saw_unknown || details.iter().all(|detail| detail.tx_status.is_none()) {
        BorrowingTransactionState::Unknown
    } else {
        BorrowingTransactionState::Succeeded
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};

    use super::*;

    #[test]
    fn supply_plan_preserves_provider_order_and_mixed_number_encodings() {
        let mut approval = models::DefiTransaction::new();
        approval.call_data_type = Some(models::defi_transaction::CallDataType::Approve);
        approval.to = Some(Some(Address::with_last_byte(1).to_string()));
        approval.data = Some("0x0102".to_owned());
        approval.value = Some(Some("0x0".to_owned()));
        approval.gas_limit = Some(Some("0x5208".to_owned()));

        let mut deposit = models::DefiTransaction::new();
        deposit.call_data_type = Some(models::defi_transaction::CallDataType::Deposit);
        deposit.to = Some(Some(Address::with_last_byte(2).to_string()));
        deposit.data = Some("0x0304".to_owned());
        deposit.value = Some(Some("0".to_owned()));
        deposit.gas_limit = Some(Some("30000".to_owned()));

        let mut network_fee = models::DefiNetworkFee::new();
        network_fee.amount = Some(Some("0.0001".to_owned()));
        network_fee.token_symbol = Some("BNB".to_owned());
        network_fee.value_usd = Some(Some("0.06".to_owned()));
        let mut fee_and_contract = models::DefiFeeAndContract::new();
        fee_and_contract.estimated_network_fee = Some(Box::new(network_fee));
        let mut health_factor = models::DefiHealthFactor::new();
        health_factor.before = Some(Some("1.8".to_owned()));
        health_factor.after = Some(Some("2.1".to_owned()));
        let mut warning = models::DefiWarning::new();
        warning.code = Some("RISK".to_owned());
        warning.level = Some("INFO".to_owned());
        warning.message = Some("Review the position".to_owned());
        let mut preview = models::DefiTransactionPreview::new();
        preview.fee_and_contract = Some(Box::new(fee_and_contract));
        preview.health_factor = Some(Box::new(health_factor));
        preview.warnings = Some(vec![warning]);

        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![approval, deposit]);
        plan.preview = Some(Box::new(preview));
        let supply = PreparedBorrowingAction::try_from(ProviderPreparedAction(plan))
            .unwrap_or_else(|error| panic!("valid supply plan rejected: {error}"));

        assert_eq!(supply.transactions.len(), 2);
        assert_eq!(
            supply.transactions[0].kind,
            BorrowingTransactionKind::Approve
        );
        assert_eq!(supply.transactions[0].gas_limit, Some(U256::from(21_000)));
        assert_eq!(
            supply.transactions[1].kind,
            BorrowingTransactionKind::Deposit
        );
        assert_eq!(supply.transactions[1].gas_limit, Some(U256::from(30_000)));
        assert_eq!(
            supply
                .preview
                .estimated_network_fee
                .as_ref()
                .map(|fee| fee.amount.to_string()),
            Some("0.0001".to_owned())
        );
        assert_eq!(
            supply
                .preview
                .estimated_network_fee
                .as_ref()
                .map(|fee| fee.token_symbol.as_str()),
            Some("BNB")
        );
        assert_eq!(
            supply
                .preview
                .estimated_network_fee
                .and_then(|fee| fee.value_usd)
                .map(|value| value.to_string()),
            Some("0.06".to_owned())
        );
        assert_eq!(
            supply
                .preview
                .health_factor_before
                .map(|value| value.to_string()),
            Some("1.8".to_owned())
        );
        assert_eq!(
            supply
                .preview
                .health_factor_after
                .map(|value| value.to_string()),
            Some("2.1".to_owned())
        );
        assert_eq!(supply.preview.warnings.len(), 1);
    }

    #[test]
    fn zero_health_factor_pair_is_not_exposed_as_liquidation_risk() {
        let mut deposit = models::DefiTransaction::new();
        deposit.call_data_type = Some(models::defi_transaction::CallDataType::Deposit);
        deposit.to = Some(Some(Address::with_last_byte(2).to_string()));
        deposit.data = Some("0x0304".to_owned());

        let mut health_factor = models::DefiHealthFactor::new();
        health_factor.before = Some(Some("0".to_owned()));
        health_factor.after = Some(Some("0".to_owned()));
        let mut preview = models::DefiTransactionPreview::new();
        preview.health_factor = Some(Box::new(health_factor));
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![deposit]);
        plan.preview = Some(Box::new(preview));

        let action = PreparedBorrowingAction::try_from(ProviderPreparedAction(plan))
            .unwrap_or_else(|error| panic!("valid supply plan rejected: {error}"));

        assert_eq!(action.preview.health_factor_before, None);
        assert_eq!(action.preview.health_factor_after, None);
    }

    #[test]
    fn transaction_details_collapse_to_one_consistent_state() {
        let mut completed = models::TransactionDetail::new();
        completed.tx_status = Some("SUCCESS".to_owned());
        completed.height = Some("123".to_owned());
        completed.tx_time = Some("1748601600000".to_owned());
        let mut pending = models::TransactionDetail::new();
        pending.tx_status = Some("PENDING".to_owned());

        let status = BorrowingTransactionStatus::try_from(ProviderTransactionDetails(vec![
            completed, pending,
        ]))
        .unwrap_or_else(|error| panic!("valid transaction details rejected: {error}"));

        assert_eq!(status.state, BorrowingTransactionState::Pending);
        assert_eq!(status.block_number, Some(123));
        assert_eq!(status.executed_at_ms, Some(1_748_601_600_000));
    }

    #[test]
    fn carry_deposit_plan_preserves_actions_and_simulation_preview() {
        let asset = Address::with_last_byte(3);
        let contract = Address::with_last_byte(4);
        let mut approval = models::DefiTransaction::new();
        approval.call_data_type = Some(models::defi_transaction::CallDataType::Approve);
        approval.to = Some(Some(Address::with_last_byte(1).to_string()));
        approval.data = Some("0x0102".to_owned());

        let mut deposit = models::DefiTransaction::new();
        deposit.call_data_type = Some(models::defi_transaction::CallDataType::Deposit);
        deposit.to = Some(Some(Address::with_last_byte(2).to_string()));
        deposit.data = Some("0x0304".to_owned());
        deposit.value = Some(Some("0x0".to_owned()));
        deposit.max_priority_fee_per_gas = Some(Some("100".to_owned()));
        deposit.max_fee_per_gas = Some(Some("200".to_owned()));

        let mut balance_change = models::DefiBalanceChange::new();
        balance_change.token_address = Some(asset.to_string());
        balance_change.token_symbol = Some("USDT".to_owned());
        balance_change.amount = Some("-25.5".to_owned());
        balance_change.value_usd = Some(Some("25.49".to_owned()));
        let mut network_fee = models::DefiNetworkFee::new();
        network_fee.amount = Some(Some("0.00002".to_owned()));
        network_fee.token_symbol = Some("BNB".to_owned());
        network_fee.value_usd = Some(Some("0.01".to_owned()));
        let mut interaction = models::DefiContract::new();
        interaction.address = Some(contract.to_string());
        let mut fee_and_contract = models::DefiFeeAndContract::new();
        fee_and_contract.estimated_network_fee = Some(Box::new(network_fee));
        fee_and_contract.interact_with = Some(Box::new(interaction));
        let mut warning = models::DefiWarning::new();
        warning.code = Some("INFO".to_owned());
        warning.level = Some("INFO".to_owned());
        warning.message = Some("Review the deposit".to_owned());
        let mut preview = models::DefiTransactionPreview::new();
        preview.balance_change = Some(vec![balance_change]);
        preview.fee_and_contract = Some(Box::new(fee_and_contract));
        preview.warnings = Some(vec![warning]);
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![approval, deposit]);
        plan.preview = Some(Box::new(preview));

        let plan = CarryDepositPlan::try_from(ProviderCarryDepositPlan(plan))
            .unwrap_or_else(|error| panic!("valid carry deposit plan rejected: {error}"));

        assert_eq!(plan.transactions.len(), 2);
        assert_eq!(plan.transactions[0].kind, CarryTransactionKind::Approve);
        assert_eq!(plan.transactions[1].kind, CarryTransactionKind::Deposit);
        assert_eq!(
            plan.transactions[1].max_priority_fee_per_gas,
            Some(U256::from(100))
        );
        assert_eq!(plan.transactions[1].max_fee_per_gas, Some(U256::from(200)));
        assert_eq!(plan.preview.balance_changes.len(), 1);
        assert_eq!(plan.preview.balance_changes[0].token.contract, asset);
        assert_eq!(plan.preview.balance_changes[0].token.symbol, "USDT");
        assert_eq!(
            plan.preview.balance_changes[0].amount,
            Decimal::new(-255, 1)
        );
        assert_eq!(
            plan.preview.balance_changes[0].value_usd,
            Some(Decimal::new(2_549, 2))
        );
        assert_eq!(plan.preview.interact_with, Some(contract));
        assert_eq!(
            plan.preview
                .estimated_network_fee
                .as_ref()
                .map(|fee| fee.amount),
            Some(Decimal::new(2, 5))
        );
        assert_eq!(plan.preview.warnings.len(), 1);
    }

    #[test]
    fn carry_deposit_plan_requires_a_final_deposit_action_and_preview() {
        let mut approval = models::DefiTransaction::new();
        approval.call_data_type = Some(models::defi_transaction::CallDataType::Approve);
        approval.to = Some(Some(Address::with_last_byte(1).to_string()));
        approval.data = Some("0x0102".to_owned());
        let mut approval_only = models::DefiTransactionPlan::new();
        approval_only.data_list = Some(vec![approval]);
        approval_only.preview = Some(Box::new(models::DefiTransactionPreview::new()));

        assert_eq!(
            CarryDepositPlan::try_from(ProviderCarryDepositPlan(approval_only)),
            Err(EarnProviderError::InvalidData)
        );

        let mut deposit = models::DefiTransaction::new();
        deposit.call_data_type = Some(models::defi_transaction::CallDataType::Deposit);
        deposit.to = Some(Some(Address::with_last_byte(2).to_string()));
        deposit.data = Some("0x0304".to_owned());
        let mut missing_preview = models::DefiTransactionPlan::new();
        missing_preview.data_list = Some(vec![deposit]);

        assert_eq!(
            CarryDepositPlan::try_from(ProviderCarryDepositPlan(missing_preview)),
            Err(EarnProviderError::InvalidData)
        );
    }
}
