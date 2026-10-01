use alloy::sol_types::SolCall;
use alloy_primitives::{Address, U256};
use stockfront_core::{
    managed_portfolios::{
        ManagedPortfolioProviderError, ManagedPortfolioTransactionKind,
        PrepareManagedPortfolioDepositRequest, PreparedManagedPortfolioDeposit,
        PreparedManagedPortfolioTransaction,
    },
    prepared_action::ActionPreview,
};

use super::{GliderAdapter, contracts::IERC20, model::BnbManagedPortfolioDepositAccount};

impl GliderAdapter {
    pub(super) async fn prepare_deposit_action(
        &self,
        portfolio_id: &str,
        request: &PrepareManagedPortfolioDepositRequest,
    ) -> Result<PreparedManagedPortfolioDeposit, ManagedPortfolioProviderError> {
        let account = BnbManagedPortfolioDepositAccount::try_from(
            *self.client.portfolio(portfolio_id).await?.data,
        )?;
        Ok(deposit_plan(account.into_inner().address, request))
    }
}

fn deposit_plan(
    account: Address,
    request: &PrepareManagedPortfolioDepositRequest,
) -> PreparedManagedPortfolioDeposit {
    PreparedManagedPortfolioDeposit {
        transactions: vec![PreparedManagedPortfolioTransaction {
            kind: ManagedPortfolioTransactionKind::Deposit,
            to: request.token,
            data: IERC20::transferCall {
                recipient: account,
                amount: request.raw_amount,
            }
            .abi_encode()
            .into(),
            value: U256::ZERO,
            gas_limit: None,
            gas_price: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        }],
        preview: ActionPreview {
            interact_with: Some(account),
            ..ActionPreview::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;

    #[test]
    fn prepares_exact_erc20_transfer_to_portfolio_account() {
        let account = address!("1111111111111111111111111111111111111111");
        let token = address!("2222222222222222222222222222222222222222");
        let raw_amount = U256::from(1_000_000_u64);

        let prepared = deposit_plan(
            account,
            &PrepareManagedPortfolioDepositRequest { token, raw_amount },
        );
        let [transaction] = prepared.transactions.as_slice() else {
            panic!("deposit must contain exactly one transaction");
        };

        assert_eq!(transaction.kind, ManagedPortfolioTransactionKind::Deposit);
        assert_eq!(transaction.to, token);
        assert_eq!(transaction.value, U256::ZERO);
        assert_eq!(
            transaction.data.as_ref(),
            IERC20::transferCall {
                recipient: account,
                amount: raw_amount,
            }
            .abi_encode()
        );
        assert_eq!(prepared.preview.interact_with, Some(account));
    }
}
