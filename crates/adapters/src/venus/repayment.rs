use alloy::sol_types::SolCall;
use alloy_primitives::U256;
use stockfront_core::{
    bstock_borrowing::{
        BorrowingProviderError, BorrowingTransactionKind, BorrowingVenue, PositionAmount,
        PrepareRepaymentRequest, PreparedBorrowingAction, PreparedBorrowingTransaction,
        RepaymentCapability, RepaymentCapabilityRequest, RepaymentCapabilityUnavailableReason,
    },
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::{
    client::VenusBorrowingAdapter,
    contracts::{IERC20, IVToken},
    conversion::decode_units,
    markets::MarketState,
};

struct RepaymentSnapshot {
    market: MarketState,
    debt: U256,
    wallet_balance: U256,
    approval_amount: U256,
    allowance: U256,
}

impl VenusBorrowingAdapter {
    pub(super) async fn repayment_capability(
        &self,
        request: &RepaymentCapabilityRequest,
    ) -> Result<RepaymentCapability, BorrowingProviderError> {
        let snapshot = self
            .repayment_snapshot(request.wallet, request.debt_token)
            .await?;
        let mut unavailable_reasons = Vec::new();
        if request.pool != self.comptroller {
            unavailable_reasons.push(RepaymentCapabilityUnavailableReason::UnsupportedPool);
        }
        if snapshot.debt == U256::ZERO {
            unavailable_reasons.push(RepaymentCapabilityUnavailableReason::NoDebt);
        }
        if snapshot.wallet_balance < snapshot.approval_amount {
            unavailable_reasons
                .push(RepaymentCapabilityUnavailableReason::InsufficientWalletBalance);
        }

        Ok(RepaymentCapability {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            pool: request.pool,
            debt: snapshot.market.token.clone(),
            debt_amount: decode_units(snapshot.debt, snapshot.market.token_decimals)?,
            wallet_balance: decode_units(snapshot.wallet_balance, snapshot.market.token_decimals)?,
            approval_amount: decode_units(
                snapshot.approval_amount,
                snapshot.market.token_decimals,
            )?,
            approval_required: snapshot.allowance < snapshot.approval_amount,
            unavailable_reasons,
            observed_at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub(super) async fn prepare_full_repayment_action(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        if request.amount != PositionAmount::Full {
            return Err(BorrowingProviderError::Rejected);
        }
        let snapshot = self
            .repayment_snapshot(request.wallet, request.debt_token)
            .await?;
        if snapshot.debt == U256::ZERO {
            return Err(BorrowingProviderError::NoDebt);
        }
        if snapshot.wallet_balance < snapshot.approval_amount {
            return Err(BorrowingProviderError::InsufficientWalletBalance);
        }

        let repay_data = IVToken::repayBorrowCall { amount: U256::MAX }
            .abi_encode()
            .into();
        if snapshot.allowance >= snapshot.approval_amount {
            return self
                .binance
                .prepare_evm_borrowing_action(
                    request.wallet,
                    BorrowingTransactionKind::Repay,
                    snapshot.market.market,
                    repay_data,
                )
                .await;
        }

        let approval_data = IERC20::approveCall {
            spender: snapshot.market.market,
            amount: snapshot.approval_amount,
        }
        .abi_encode()
        .into();
        let mut action = self
            .binance
            .prepare_evm_borrowing_action(
                request.wallet,
                BorrowingTransactionKind::Approve,
                request.debt_token,
                approval_data,
            )
            .await?;
        if action.transactions.len() != 1
            || action.transactions[0].kind != BorrowingTransactionKind::Approve
        {
            return Err(BorrowingProviderError::InvalidData);
        }
        action.transactions.push(PreparedBorrowingTransaction {
            kind: BorrowingTransactionKind::Repay,
            to: snapshot.market.market,
            data: repay_data,
            value: U256::ZERO,
            gas_limit: None,
            gas_price: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        });
        Ok(action)
    }

    async fn repayment_snapshot(
        &self,
        wallet: WalletAddress,
        debt_token: alloy_primitives::Address,
    ) -> Result<RepaymentSnapshot, BorrowingProviderError> {
        let market = self.market_state(debt_token).await?;
        let debt = IVToken::new(market.market, &self.provider)
            .borrowBalanceCurrent(wallet.as_address())
            .call()
            .await
            .map_err(|_| BorrowingProviderError::Unavailable)?;
        let token = IERC20::new(debt_token, &self.provider);
        let (wallet_balance, allowance) = tokio::try_join!(
            async {
                token
                    .balanceOf(wallet.as_address())
                    .call()
                    .await
                    .map_err(|_| BorrowingProviderError::Unavailable)
            },
            async {
                token
                    .allowance(wallet.as_address(), market.market)
                    .call()
                    .await
                    .map_err(|_| BorrowingProviderError::Unavailable)
            }
        )?;
        Ok(RepaymentSnapshot {
            market,
            debt,
            wallet_balance,
            approval_amount: buffered_repayment(debt)?,
            allowance,
        })
    }
}

pub(super) fn buffered_repayment(debt: U256) -> Result<U256, BorrowingProviderError> {
    const SCALE: u64 = 100_000;
    debt.checked_mul(U256::from(SCALE + 1))
        .and_then(|value| value.checked_add(U256::from(SCALE - 1)))
        .and_then(|value| value.checked_div(U256::from(SCALE)))
        .ok_or(BorrowingProviderError::InvalidData)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_repayment_approval_uses_the_venus_buffer() {
        assert_eq!(
            buffered_repayment(U256::from(1_000_000)),
            Ok(U256::from(1_000_010))
        );
        assert_eq!(buffered_repayment(U256::from(1)), Ok(U256::from(2)));
    }
}
