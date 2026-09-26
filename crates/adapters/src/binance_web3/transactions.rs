use alloy_primitives::{Address, Bytes, U256};
use binance_web3_rwa::{
    apis::transaction_api::{
        GetGasLimitParams, GetGasPriceParams, SimulateTransactionParams, get_gas_limit,
        get_gas_price, simulate_transaction,
    },
    models,
};
use stockfront_core::{
    bstock_borrowing::{
        BorrowingPreview, BorrowingProviderError, BorrowingTransactionKind,
        PreparedBorrowingAction, PreparedBorrowingTransaction,
    },
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::{
    client::BinanceWeb3Client,
    provider::{ensure_business_success, map_request_error, parse, required},
};

impl BinanceWeb3Client {
    pub(crate) async fn prepare_evm_borrowing_action(
        &self,
        wallet: WalletAddress,
        kind: BorrowingTransactionKind,
        to: Address,
        data: Bytes,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let transaction = models::EvmTransaction::new(
            wallet.as_address().to_string(),
            to.to_string(),
            "0".to_owned(),
            data.to_string(),
        );
        let request = models::EvmPreTransactionRequest::new(BNB_CHAIN_ID.to_string(), transaction);
        let simulation = simulate_transaction(
            &self.configuration,
            SimulateTransactionParams {
                evm_pre_transaction_request: request.clone(),
            },
        )
        .await
        .map_err(map_request_error)?;
        ensure_business_success(simulation.code, simulation.success)?;
        if !simulation
            .data
            .and_then(|result| result.status)
            .is_some_and(|status| status.eq_ignore_ascii_case("SUCCESS"))
        {
            return Err(BorrowingProviderError::Rejected);
        }

        let (gas_limit, gas_price) = tokio::try_join!(
            async {
                get_gas_limit(
                    &self.configuration,
                    GetGasLimitParams {
                        evm_pre_transaction_request: request,
                    },
                )
                .await
                .map_err(map_request_error)
            },
            async {
                get_gas_price(
                    &self.configuration,
                    GetGasPriceParams {
                        binance_chain_id: BNB_CHAIN_ID.to_string(),
                    },
                )
                .await
                .map_err(map_request_error)
            },
        )?;
        ensure_business_success(gas_limit.code, gas_limit.success)?;
        ensure_business_success(gas_price.code, gas_price.success)?;

        let gas_limit = required(gas_limit.data.and_then(|data| data.gas_limit))?;
        let gas_limit = parse(&gas_limit)?;
        let gas_price = required(
            gas_price
                .data
                .and_then(|data| data.evm_legacy_gas_price)
                .and_then(|price| price.medium_gas_price),
        )?;
        let gas_price = parse(&gas_price)?;

        Ok(PreparedBorrowingAction {
            transactions: vec![PreparedBorrowingTransaction {
                kind,
                to,
                data,
                value: U256::ZERO,
                gas_limit: Some(gas_limit),
                gas_price: Some(gas_price),
                max_priority_fee_per_gas: None,
                max_fee_per_gas: None,
            }],
            preview: BorrowingPreview::default(),
        })
    }
}
