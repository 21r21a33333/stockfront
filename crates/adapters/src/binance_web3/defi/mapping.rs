use binance_web3_rwa::models;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingProviderError, BorrowingTransactionState, BorrowingTransactionStatus,
    },
    carry::{
        CarryActionPlan, CarryActionPreview, CarryTransactionKind, EarnProviderError,
        RedemptionDelay,
    },
    liquidity::{
        LiquidityAmount, LiquidityProviderError, LiquidityTransactionKind, PreparedLiquidityAction,
        PreparedLiquidityEntry,
    },
    prepared_action::{
        ActionPreview, ActionWarning, AssetBalanceChange, NetworkFee, PreparedTransaction,
    },
};

use super::gateway::LpEntryPlan;

use super::super::provider::{
    InvalidProviderData, parse, parse_optional, parse_optional_u256, required,
};

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

impl TryFrom<ProviderCarryDepositPlan> for CarryActionPlan {
    type Error = EarnProviderError;

    fn try_from(value: ProviderCarryDepositPlan) -> Result<Self, Self::Error> {
        carry_action_plan(value.0, CarryTransactionKind::Deposit)
    }
}

pub(super) struct ProviderCarryWithdrawalPlan(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderCarryWithdrawalPlan> for CarryActionPlan {
    type Error = EarnProviderError;

    fn try_from(value: ProviderCarryWithdrawalPlan) -> Result<Self, Self::Error> {
        carry_action_plan(value.0, CarryTransactionKind::Redeem)
    }
}

pub(super) struct ProviderCarryClaimPlan(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderCarryClaimPlan> for CarryActionPlan {
    type Error = EarnProviderError;

    fn try_from(value: ProviderCarryClaimPlan) -> Result<Self, Self::Error> {
        carry_action_plan(value.0, CarryTransactionKind::Claim)
    }
}

fn carry_action_plan(
    plan: models::DefiTransactionPlan,
    final_kind: CarryTransactionKind,
) -> Result<CarryActionPlan, EarnProviderError> {
    let transactions = prepared_transactions(plan.data_list, |kind| match kind {
        models::defi_transaction::CallDataType::Approve => Some(CarryTransactionKind::Approve),
        models::defi_transaction::CallDataType::Deposit => Some(CarryTransactionKind::Deposit),
        models::defi_transaction::CallDataType::Redeem => Some(CarryTransactionKind::Redeem),
        models::defi_transaction::CallDataType::Claim => Some(CarryTransactionKind::Claim),
        _ => None,
    })?;
    if transactions.last().map(|transaction| transaction.kind) != Some(final_kind) {
        return Err(EarnProviderError::InvalidData);
    }
    let redemption_delay = redemption_delay(plan.redeem_delay_days)?;
    let preview = required(plan.preview)?;
    let action = action_preview(&preview)?;
    let preview = CarryActionPreview {
        action,
        redemption_delay,
    };

    Ok(CarryActionPlan {
        transactions,
        preview,
    })
}

fn action_preview(
    preview: &models::DefiTransactionPreview,
) -> Result<ActionPreview, InvalidProviderData> {
    let balance_changes = preview
        .balance_change
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|change| change.token_symbol.clone().map(|symbol| (change, symbol)))
        .map(|(change, symbol)| {
            Ok(AssetBalanceChange {
                token: stockfront_core::bstock_borrowing::BorrowingToken {
                    contract: parse(&required(change.token_address.clone())?)?,
                    symbol,
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
    Ok(ActionPreview {
        balance_changes,
        estimated_network_fee: network_fee(preview)?,
        interact_with,
        warnings: action_warnings(preview),
    })
}

pub(super) struct ProviderLiquidityEntryPlan(pub(super) LpEntryPlan);

impl TryFrom<ProviderLiquidityEntryPlan> for PreparedLiquidityEntry {
    type Error = LiquidityProviderError;

    fn try_from(value: ProviderLiquidityEntryPlan) -> Result<Self, Self::Error> {
        let plan = value.0;
        let required_amounts = plan
            .token_amounts
            .into_iter()
            .map(|token| {
                Ok(LiquidityAmount {
                    token: parse(&token.token_address)?,
                    amount: parse(&token.amount)?,
                })
            })
            .collect::<Result<Vec<_>, InvalidProviderData>>()?;
        let transactions = prepared_transactions(plan.transactions.data_list, |kind| match kind {
            models::defi_transaction::CallDataType::Approve => {
                Some(LiquidityTransactionKind::Approve)
            }
            models::defi_transaction::CallDataType::LpAdd => Some(LiquidityTransactionKind::LpAdd),
            _ => None,
        })?;
        if transactions.last().map(|transaction| transaction.kind)
            != Some(LiquidityTransactionKind::LpAdd)
        {
            return Err(LiquidityProviderError::InvalidData);
        }
        let preview = required(plan.transactions.preview)?;
        let preview = action_preview(&preview)?;

        Ok(Self {
            required_amounts,
            transactions,
            preview,
        })
    }
}

pub(super) struct ProviderLiquidityClaimPlan(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderLiquidityClaimPlan> for PreparedLiquidityAction {
    type Error = LiquidityProviderError;

    fn try_from(value: ProviderLiquidityClaimPlan) -> Result<Self, Self::Error> {
        prepared_liquidity_action(value.0, LiquidityTransactionKind::Claim)
    }
}

pub(super) struct ProviderLiquidityWithdrawalPlan(pub(super) models::DefiTransactionPlan);

impl TryFrom<ProviderLiquidityWithdrawalPlan> for PreparedLiquidityAction {
    type Error = LiquidityProviderError;

    fn try_from(value: ProviderLiquidityWithdrawalPlan) -> Result<Self, Self::Error> {
        prepared_liquidity_action(value.0, LiquidityTransactionKind::LpRemove)
    }
}

fn prepared_liquidity_action(
    plan: models::DefiTransactionPlan,
    expected: LiquidityTransactionKind,
) -> Result<PreparedLiquidityAction, LiquidityProviderError> {
    let transactions = prepared_transactions(plan.data_list, |kind| match kind {
        models::defi_transaction::CallDataType::Approve => Some(LiquidityTransactionKind::Approve),
        models::defi_transaction::CallDataType::LpRemove => {
            Some(LiquidityTransactionKind::LpRemove)
        }
        models::defi_transaction::CallDataType::Claim => Some(LiquidityTransactionKind::Claim),
        _ => None,
    })?;
    if transactions.last().map(|transaction| transaction.kind) != Some(expected) {
        return Err(LiquidityProviderError::InvalidData);
    }
    let preview = required(plan.preview)?;
    Ok(PreparedLiquidityAction {
        transactions,
        preview: action_preview(&preview)?,
    })
}

fn redemption_delay(
    days: Option<Vec<String>>,
) -> Result<Option<RedemptionDelay>, InvalidProviderData> {
    let days = days.unwrap_or_default();
    match days.as_slice() {
        [] => Ok(None),
        [days] => {
            let days = parse(days)?;
            Ok(Some(RedemptionDelay {
                min_days: days,
                max_days: days,
            }))
        }
        [min_days, max_days] => Ok(Some(RedemptionDelay {
            min_days: parse(min_days)?,
            max_days: parse(max_days)?,
        })),
        _ => Err(InvalidProviderData),
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
    use rust_decimal::Decimal;

    use super::*;

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

        let plan = CarryActionPlan::try_from(ProviderCarryDepositPlan(plan))
            .unwrap_or_else(|error| panic!("valid carry deposit plan rejected: {error}"));

        assert_eq!(plan.transactions.len(), 2);
        assert_eq!(plan.transactions[0].kind, CarryTransactionKind::Approve);
        assert_eq!(plan.transactions[1].kind, CarryTransactionKind::Deposit);
        assert_eq!(
            plan.transactions[1].max_priority_fee_per_gas,
            Some(U256::from(100))
        );
        assert_eq!(plan.transactions[1].max_fee_per_gas, Some(U256::from(200)));
        assert_eq!(plan.preview.action.balance_changes.len(), 1);
        assert_eq!(plan.preview.action.balance_changes[0].token.contract, asset);
        assert_eq!(plan.preview.action.balance_changes[0].token.symbol, "USDT");
        assert_eq!(
            plan.preview.action.balance_changes[0].amount,
            Decimal::new(-255, 1)
        );
        assert_eq!(
            plan.preview.action.balance_changes[0].value_usd,
            Some(Decimal::new(2_549, 2))
        );
        assert_eq!(plan.preview.action.interact_with, Some(contract));
        assert_eq!(
            plan.preview
                .action
                .estimated_network_fee
                .as_ref()
                .map(|fee| fee.amount),
            Some(Decimal::new(2, 5))
        );
        assert_eq!(plan.preview.action.warnings.len(), 1);
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
            CarryActionPlan::try_from(ProviderCarryDepositPlan(approval_only)),
            Err(EarnProviderError::InvalidData)
        );

        let mut deposit = models::DefiTransaction::new();
        deposit.call_data_type = Some(models::defi_transaction::CallDataType::Deposit);
        deposit.to = Some(Some(Address::with_last_byte(2).to_string()));
        deposit.data = Some("0x0304".to_owned());
        let mut missing_preview = models::DefiTransactionPlan::new();
        missing_preview.data_list = Some(vec![deposit]);

        assert_eq!(
            CarryActionPlan::try_from(ProviderCarryDepositPlan(missing_preview)),
            Err(EarnProviderError::InvalidData)
        );
    }

    #[test]
    fn liquidity_entry_maps_required_amounts_actions_and_preview() {
        let input = Address::with_last_byte(1);
        let paired = Address::with_last_byte(2);
        let contract = Address::with_last_byte(3);
        let mut approval = models::DefiTransaction::new();
        approval.call_data_type = Some(models::defi_transaction::CallDataType::Approve);
        approval.to = Some(Some(Address::with_last_byte(4).to_string()));
        approval.data = Some("0x0102".to_owned());
        let mut lp_add = models::DefiTransaction::new();
        lp_add.call_data_type = Some(models::defi_transaction::CallDataType::LpAdd);
        lp_add.to = Some(Some(contract.to_string()));
        lp_add.data = Some("0x0304".to_owned());

        let mut balance_change = models::DefiBalanceChange::new();
        balance_change.token_address = Some(input.to_string());
        balance_change.token_symbol = Some("NVDAB".to_owned());
        balance_change.amount = Some("-0.01".to_owned());
        let mut nft_change = models::DefiBalanceChange::new();
        nft_change.token_address = Some(contract.to_string());
        nft_change.amount = Some("1".to_owned());
        nft_change.value_usd = Some(None);
        let mut interaction = models::DefiContract::new();
        interaction.address = Some(contract.to_string());
        let mut fee_and_contract = models::DefiFeeAndContract::new();
        fee_and_contract.interact_with = Some(Box::new(interaction));
        let mut preview = models::DefiTransactionPreview::new();
        preview.balance_change = Some(vec![balance_change, nft_change]);
        preview.fee_and_contract = Some(Box::new(fee_and_contract));
        let mut transactions = models::DefiTransactionPlan::new();
        transactions.data_list = Some(vec![approval, lp_add]);
        transactions.preview = Some(Box::new(preview));
        let provider = LpEntryPlan {
            token_amounts: vec![
                models::DefiTokenAmount::new(input.to_string(), "0.01".to_owned()),
                models::DefiTokenAmount::new(paired.to_string(), "2.5".to_owned()),
            ],
            transactions,
        };

        let entry = PreparedLiquidityEntry::try_from(ProviderLiquidityEntryPlan(provider))
            .unwrap_or_else(|error| panic!("valid liquidity entry rejected: {error}"));

        assert_eq!(
            entry.required_amounts,
            vec![
                LiquidityAmount {
                    token: input,
                    amount: Decimal::new(1, 2),
                },
                LiquidityAmount {
                    token: paired,
                    amount: Decimal::new(25, 1),
                },
            ]
        );
        assert_eq!(entry.transactions.len(), 2);
        assert_eq!(
            entry.transactions[0].kind,
            LiquidityTransactionKind::Approve
        );
        assert_eq!(entry.transactions[1].kind, LiquidityTransactionKind::LpAdd);
        assert_eq!(entry.preview.interact_with, Some(contract));
        assert_eq!(entry.preview.balance_changes.len(), 1);
    }

    #[test]
    fn liquidity_claim_maps_the_claim_action_and_preview() {
        let token = Address::with_last_byte(1);
        let contract = Address::with_last_byte(2);
        let mut claim = models::DefiTransaction::new();
        claim.call_data_type = Some(models::defi_transaction::CallDataType::Claim);
        claim.to = Some(Some(contract.to_string()));
        claim.data = Some("0x0102".to_owned());

        let mut balance_change = models::DefiBalanceChange::new();
        balance_change.token_address = Some(token.to_string());
        balance_change.token_symbol = Some("USDT".to_owned());
        balance_change.amount = Some("0.25".to_owned());
        let mut preview = models::DefiTransactionPreview::new();
        preview.balance_change = Some(vec![balance_change]);
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![claim]);
        plan.preview = Some(Box::new(preview));

        let action = PreparedLiquidityAction::try_from(ProviderLiquidityClaimPlan(plan))
            .unwrap_or_else(|error| panic!("valid liquidity claim rejected: {error}"));

        assert_eq!(action.transactions.len(), 1);
        assert_eq!(action.transactions[0].kind, LiquidityTransactionKind::Claim);
        assert_eq!(action.preview.balance_changes.len(), 1);
        assert_eq!(action.preview.balance_changes[0].token.contract, token);
        assert_eq!(
            action.preview.balance_changes[0].amount,
            Decimal::new(25, 2)
        );
    }

    #[test]
    fn liquidity_withdrawal_maps_the_remove_action_and_returned_assets() {
        let token = Address::with_last_byte(1);
        let contract = Address::with_last_byte(2);
        let mut remove = models::DefiTransaction::new();
        remove.call_data_type = Some(models::defi_transaction::CallDataType::LpRemove);
        remove.to = Some(Some(contract.to_string()));
        remove.data = Some("0x0102".to_owned());

        let mut balance_change = models::DefiBalanceChange::new();
        balance_change.token_address = Some(token.to_string());
        balance_change.token_symbol = Some("USDT".to_owned());
        balance_change.amount = Some("2.5".to_owned());
        let mut preview = models::DefiTransactionPreview::new();
        preview.balance_change = Some(vec![balance_change]);
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![remove]);
        plan.preview = Some(Box::new(preview));

        let action = PreparedLiquidityAction::try_from(ProviderLiquidityWithdrawalPlan(plan))
            .unwrap_or_else(|error| panic!("valid liquidity withdrawal rejected: {error}"));

        assert_eq!(action.transactions.len(), 1);
        assert_eq!(
            action.transactions[0].kind,
            LiquidityTransactionKind::LpRemove
        );
        assert_eq!(action.preview.balance_changes.len(), 1);
        assert_eq!(action.preview.balance_changes[0].token.contract, token);
        assert_eq!(
            action.preview.balance_changes[0].amount,
            Decimal::new(25, 1)
        );
    }

    #[test]
    fn carry_withdrawal_plan_maps_ordered_actions_and_redemption_delay() {
        let mut approval = models::DefiTransaction::new();
        approval.call_data_type = Some(models::defi_transaction::CallDataType::Approve);
        approval.to = Some(Some(Address::with_last_byte(1).to_string()));
        approval.data = Some("0x0102".to_owned());
        let mut redeem = models::DefiTransaction::new();
        redeem.call_data_type = Some(models::defi_transaction::CallDataType::Redeem);
        redeem.to = Some(Some(Address::with_last_byte(2).to_string()));
        redeem.data = Some("0x0304".to_owned());
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![approval, redeem]);
        plan.preview = Some(Box::new(models::DefiTransactionPreview::new()));
        plan.redeem_delay_days = Some(vec!["7".to_owned(), "10".to_owned()]);

        let plan = CarryActionPlan::try_from(ProviderCarryWithdrawalPlan(plan))
            .unwrap_or_else(|error| panic!("valid withdrawal plan rejected: {error}"));

        assert_eq!(plan.transactions.len(), 2);
        assert_eq!(plan.transactions[0].kind, CarryTransactionKind::Approve);
        assert_eq!(plan.transactions[1].kind, CarryTransactionKind::Redeem);
        assert_eq!(
            plan.preview.redemption_delay,
            Some(RedemptionDelay {
                min_days: Decimal::new(7, 0),
                max_days: Decimal::new(10, 0),
            })
        );
    }

    #[test]
    fn carry_withdrawal_plan_maps_instant_and_fixed_redemption() {
        let plan = withdrawal_plan(None);
        let instant = CarryActionPlan::try_from(ProviderCarryWithdrawalPlan(plan))
            .unwrap_or_else(|error| panic!("instant withdrawal rejected: {error}"));
        let plan = withdrawal_plan(Some(vec!["3".to_owned()]));
        let fixed = CarryActionPlan::try_from(ProviderCarryWithdrawalPlan(plan))
            .unwrap_or_else(|error| panic!("fixed withdrawal rejected: {error}"));

        assert_eq!(instant.preview.redemption_delay, None);
        assert_eq!(
            fixed.preview.redemption_delay,
            Some(RedemptionDelay {
                min_days: Decimal::new(3, 0),
                max_days: Decimal::new(3, 0),
            })
        );
    }

    #[test]
    fn carry_claim_plan_requires_a_final_claim_action() {
        let mut claim = models::DefiTransaction::new();
        claim.call_data_type = Some(models::defi_transaction::CallDataType::Claim);
        claim.to = Some(Some(Address::with_last_byte(2).to_string()));
        claim.data = Some("0x0304".to_owned());
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![claim]);
        plan.preview = Some(Box::new(models::DefiTransactionPreview::new()));

        let plan = CarryActionPlan::try_from(ProviderCarryClaimPlan(plan))
            .unwrap_or_else(|error| panic!("valid claim plan rejected: {error}"));

        assert_eq!(plan.transactions.len(), 1);
        assert_eq!(plan.transactions[0].kind, CarryTransactionKind::Claim);
        assert_eq!(plan.preview, CarryActionPreview::default());
    }

    fn withdrawal_plan(delay: Option<Vec<String>>) -> models::DefiTransactionPlan {
        let mut redeem = models::DefiTransaction::new();
        redeem.call_data_type = Some(models::defi_transaction::CallDataType::Redeem);
        redeem.to = Some(Some(Address::with_last_byte(2).to_string()));
        redeem.data = Some("0x0304".to_owned());
        let mut plan = models::DefiTransactionPlan::new();
        plan.data_list = Some(vec![redeem]);
        plan.preview = Some(Box::new(models::DefiTransactionPreview::new()));
        plan.redeem_delay_days = delay;
        plan
    }
}
