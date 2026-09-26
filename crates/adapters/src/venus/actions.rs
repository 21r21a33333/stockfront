use alloy::sol_types::SolCall;
use alloy_primitives::B256;
use async_trait::async_trait;
use stockfront_core::bstock_borrowing::{
    BorrowingActionGateway, BorrowingPreview, BorrowingProviderError, BorrowingTransactionKind,
    BorrowingTransactionStatus, PrepareBorrowRequest, PrepareCollateralRequest,
    PrepareSupplyRequest, PreparedBorrowingAction,
};

use super::{
    client::VenusBorrowingAdapter,
    contracts::{IVToken, IVenusComptroller},
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
            return Ok(PreparedBorrowingAction {
                transactions: Vec::new(),
                preview: BorrowingPreview::default(),
            });
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

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
        self.binance
            .borrowing_transaction_status(transaction_hash)
            .await
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

        assert_eq!(&collateral[..4], &[0xc2, 0x99, 0x82, 0x38]);
        assert_eq!(&borrow[..4], &[0xc5, 0xeb, 0xea, 0xec]);
    }
}
