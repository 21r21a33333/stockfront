use alloy::sol_types::SolCall;
use alloy_primitives::{B256, U256};
use async_trait::async_trait;
use stockfront_core::bstock_borrowing::{
    BorrowingActionGateway, BorrowingPreview, BorrowingProviderError, BorrowingTransactionKind,
    BorrowingTransactionStatus, PositionAmount, PrepareBorrowRequest, PrepareCollateralRequest,
    PrepareRepaymentRequest, PrepareSupplyRequest, PrepareWithdrawalRequest,
    PreparedBorrowingAction,
};

use super::{
    client::VenusBorrowingAdapter,
    contracts::{IERC20, IVToken, IVenusComptroller},
    conversion::encode_units,
};

#[async_trait]
impl BorrowingActionGateway for VenusBorrowingAdapter {
    async fn prepare_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        self.binance.prepare_venus_supply(request).await
    }

    async fn prepare_collateral(
        &self,
        request: &PrepareCollateralRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let market = self.market_state(request.collateral_token).await?;
        if !market.accepts_collateral() {
            return Err(BorrowingProviderError::Rejected);
        }
        let enabled = IVenusComptroller::new(self.comptroller, &self.provider)
            .checkMembership(request.wallet.as_address(), market.market)
            .call()
            .await
            .map_err(|_| BorrowingProviderError::Unavailable)?;
        if enabled {
            return Ok(empty_action());
        }

        let data = IVenusComptroller::enterMarketsCall {
            vTokens: vec![market.market],
        }
        .abi_encode()
        .into();
        self.binance
            .prepare_evm_borrowing_action(
                request.wallet,
                BorrowingTransactionKind::EnableCollateral,
                self.comptroller,
                data,
            )
            .await
    }

    async fn prepare_borrow(
        &self,
        request: &PrepareBorrowRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let market = self.market_state(request.debt_token).await?;
        if !market.accepts_borrow() {
            return Err(BorrowingProviderError::Rejected);
        }
        let amount = encode_units(request.amount, market.token_decimals)?;
        if amount > market.available_borrow_liquidity() {
            return Err(BorrowingProviderError::InsufficientBorrowLiquidity);
        }
        let data = IVToken::borrowCall { amount }.abi_encode().into();
        self.binance
            .prepare_evm_borrowing_action(
                request.wallet,
                BorrowingTransactionKind::Borrow,
                market.market,
                data,
            )
            .await
    }

    async fn prepare_repayment_approval(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let market = self.market_state(request.debt_token).await?;
        let debt = self.repayment_amount(request, &market).await?;
        let approval = match &request.amount {
            PositionAmount::Exact { .. } => debt,
            PositionAmount::Full => buffered_repayment(debt)?,
        };
        let allowance = IERC20::new(request.debt_token, &self.provider)
            .allowance(request.wallet.as_address(), market.market)
            .call()
            .await
            .map_err(|_| BorrowingProviderError::Unavailable)?;
        if allowance >= approval {
            return Ok(empty_action());
        }

        let data = IERC20::approveCall {
            spender: market.market,
            amount: approval,
        }
        .abi_encode()
        .into();
        self.binance
            .prepare_evm_borrowing_action(
                request.wallet,
                BorrowingTransactionKind::Approve,
                request.debt_token,
                data,
            )
            .await
    }

    async fn prepare_repayment(
        &self,
        request: &PrepareRepaymentRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let market = self.market_state(request.debt_token).await?;
        let debt = self.repayment_amount(request, &market).await?;
        let allowance = IERC20::new(request.debt_token, &self.provider)
            .allowance(request.wallet.as_address(), market.market)
            .call()
            .await
            .map_err(|_| BorrowingProviderError::Unavailable)?;
        if allowance < debt {
            return Err(BorrowingProviderError::RepaymentApprovalRequired);
        }

        let amount = match &request.amount {
            PositionAmount::Exact { .. } => debt,
            PositionAmount::Full => U256::MAX,
        };
        let data = IVToken::repayBorrowCall { amount }.abi_encode().into();
        self.binance
            .prepare_evm_borrowing_action(
                request.wallet,
                BorrowingTransactionKind::Repay,
                market.market,
                data,
            )
            .await
    }

    async fn prepare_withdrawal(
        &self,
        request: &PrepareWithdrawalRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        self.binance.prepare_venus_withdrawal(request).await
    }

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
        self.binance
            .borrowing_transaction_status(transaction_hash)
            .await
    }
}

impl VenusBorrowingAdapter {
    async fn repayment_amount(
        &self,
        request: &PrepareRepaymentRequest,
        market: &super::markets::MarketState,
    ) -> Result<U256, BorrowingProviderError> {
        let amount = match &request.amount {
            PositionAmount::Exact { value } => encode_units(*value, market.token_decimals)?,
            PositionAmount::Full => IVToken::new(market.market, &self.provider)
                .borrowBalanceCurrent(request.wallet.as_address())
                .call()
                .await
                .map_err(|_| BorrowingProviderError::Unavailable)?,
        };
        if amount == U256::ZERO {
            return Err(BorrowingProviderError::Rejected);
        }
        Ok(amount)
    }
}

fn buffered_repayment(debt: U256) -> Result<U256, BorrowingProviderError> {
    const SCALE: u64 = 100_000;
    debt.checked_mul(U256::from(SCALE + 1))
        .and_then(|value| value.checked_add(U256::from(SCALE - 1)))
        .and_then(|value| value.checked_div(U256::from(SCALE)))
        .ok_or(BorrowingProviderError::InvalidData)
}

fn empty_action() -> PreparedBorrowingAction {
    PreparedBorrowingAction {
        transactions: Vec::new(),
        preview: BorrowingPreview::default(),
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::U256;

    use super::*;

    #[test]
    fn venus_actions_encode_the_official_calls() {
        let market = alloy_primitives::Address::with_last_byte(7);
        let collateral = IVenusComptroller::enterMarketsCall {
            vTokens: vec![market],
        }
        .abi_encode();
        let borrow = IVToken::borrowCall {
            amount: U256::from(1_000_000),
        }
        .abi_encode();
        let repay = IVToken::repayBorrowCall { amount: U256::MAX }.abi_encode();

        assert_eq!(&collateral[..4], &[0xc2, 0x99, 0x82, 0x38]);
        assert_eq!(&borrow[..4], &[0xc5, 0xeb, 0xea, 0xec]);
        assert_eq!(&repay[..4], &[0x0e, 0x75, 0x27, 0x02]);
    }

    #[test]
    fn full_repayment_approval_uses_the_venus_buffer() {
        assert_eq!(
            buffered_repayment(U256::from(1_000_000)),
            Ok(U256::from(1_000_010))
        );
        assert_eq!(buffered_repayment(U256::from(1)), Ok(U256::from(2)));
    }
}
