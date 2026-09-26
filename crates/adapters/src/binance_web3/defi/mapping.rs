use binance_web3_rwa::models;
use rust_decimal::Decimal;
use stockfront_core::bstock_borrowing::{
    BorrowingNetworkFee, BorrowingPreview, BorrowingProviderError, BorrowingTransactionKind,
    BorrowingTransactionState, BorrowingTransactionStatus, BorrowingWarning,
    PreparedBorrowingAction, PreparedBorrowingTransaction,
};

use super::super::provider::{parse, parse_optional, parse_optional_u256, required};

pub(super) struct ProviderPreparedAction(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderPreparedAction> for PreparedBorrowingAction {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderPreparedAction) -> Result<Self, Self::Error> {
        let plan = value.0;
        let transactions = plan
            .data_list
            .filter(|transactions| !transactions.is_empty())
            .ok_or(BorrowingProviderError::InvalidData)?
            .into_iter()
            .map(ProviderActionTransaction)
            .map(PreparedBorrowingTransaction::try_from)
            .collect::<Result<Vec<_>, _>>()?;
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

struct ProviderActionTransaction(models::DefiTransaction);

impl TryFrom<ProviderActionTransaction> for PreparedBorrowingTransaction {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderActionTransaction) -> Result<Self, Self::Error> {
        let value = value.0;
        let to = required(value.to.flatten())?;
        let data = required(value.data)?;
        Ok(Self {
            kind: match required(value.call_data_type)? {
                models::defi_transaction::CallDataType::Approve => {
                    BorrowingTransactionKind::Approve
                }
                models::defi_transaction::CallDataType::Deposit => {
                    BorrowingTransactionKind::Deposit
                }
                _ => return Err(BorrowingProviderError::InvalidData),
            },
            to: parse(&to)?,
            data: parse(&data)?,
            value: parse_optional_u256(value.value.flatten().as_deref())?.unwrap_or_default(),
            gas_limit: parse_optional_u256(value.gas_limit.flatten().as_deref())?,
            gas_price: parse_optional_u256(value.gas_price.flatten().as_deref())?,
            max_priority_fee_per_gas: parse_optional_u256(
                value.max_priority_fee_per_gas.flatten().as_deref(),
            )?,
            max_fee_per_gas: parse_optional_u256(value.max_fee_per_gas.flatten().as_deref())?,
        })
    }
}

struct ProviderActionPreview(models::DefiTransactionPreview);

impl TryFrom<ProviderActionPreview> for BorrowingPreview {
    type Error = BorrowingProviderError;

    fn try_from(value: ProviderActionPreview) -> Result<Self, Self::Error> {
        let value = value.0;
        let estimated_network_fee = value
            .fee_and_contract
            .and_then(|fee| fee.estimated_network_fee)
            .map(|fee| -> Result<_, BorrowingProviderError> {
                let amount = parse(&required(fee.amount.flatten())?)?;
                let token_symbol = required(fee.token_symbol)?;
                let value_usd = parse_optional(fee.value_usd.flatten().as_deref())?;
                Ok(BorrowingNetworkFee {
                    amount,
                    token_symbol,
                    value_usd,
                })
            })
            .transpose()?;
        let (health_factor_before, health_factor_after) = value
            .health_factor
            .map(|health_factor| -> Result<_, BorrowingProviderError> {
                let before: Option<Decimal> =
                    parse_optional(health_factor.before.flatten().as_deref())?;
                let after: Option<Decimal> =
                    parse_optional(health_factor.after.flatten().as_deref())?;
                Ok((before, after))
            })
            .transpose()?
            .unwrap_or_default();
        let (health_factor_before, health_factor_after) =
            normalize_health_factors(health_factor_before, health_factor_after);
        let warnings = value
            .warnings
            .unwrap_or_default()
            .into_iter()
            .filter_map(|warning| {
                Some(BorrowingWarning {
                    code: warning.code?,
                    level: warning.level?,
                    message: warning.message?,
                })
            })
            .collect();
        Ok(Self {
            estimated_network_fee,
            health_factor_before,
            health_factor_after,
            warnings,
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
}
