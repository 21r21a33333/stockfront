use alloy_primitives::B256;
use async_trait::async_trait;
use binance_web3_rwa::{
    apis::{
        de_fi_data_api::{ListDefiInvestmentsParams, list_defi_investments},
        de_fi_transaction_api::{
            BuildDefiDepositTransactionParams, build_defi_deposit_transaction,
        },
        wallet_api::{GetTransactionDetailByHashParams, get_transaction_detail_by_hash},
    },
    models,
};
use stockfront_core::{
    bstock_borrowing::{
        BorrowingActionGateway, BorrowingProviderError, BorrowingTransactionStatus,
        PrepareSupplyRequest, PreparedSupply,
    },
    bstock_market::BNB_CHAIN_ID,
};

use super::{
    super::client::{BinanceWeb3Client, is_business_success},
    mapping::{ProviderPreparedSupply, ProviderTransactionDetails},
};

const VENUS_PROTOCOL_ID: &str = "venus";

#[async_trait]
impl BorrowingActionGateway for BinanceWeb3Client {
    async fn prepare_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedSupply, BorrowingProviderError> {
        let investment_id = self.venus_investment_id(request).await?;
        let token = models::DefiTokenAmount::new(
            request.collateral_token.to_string(),
            request.amount.to_string(),
        );
        let mut body = models::BuildDefiDepositRequest::new(
            request.wallet.as_address().to_string(),
            investment_id,
            token,
        );
        body.simulate = Some(true);

        let response = build_defi_deposit_transaction(
            &self.configuration,
            BuildDefiDepositTransactionParams {
                build_defi_deposit_request: body,
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_business_success(response.code, response.success)?;
        let plan = response.data.ok_or(BorrowingProviderError::InvalidData)?;
        PreparedSupply::try_from(ProviderPreparedSupply(*plan))
    }

    async fn transaction_status(
        &self,
        transaction_hash: B256,
    ) -> Result<BorrowingTransactionStatus, BorrowingProviderError> {
        let response = get_transaction_detail_by_hash(
            &self.configuration,
            GetTransactionDetailByHashParams {
                binance_chain_id: BNB_CHAIN_ID.to_string(),
                tx_hash: transaction_hash.to_string(),
            },
        )
        .await
        .map_err(map_transaction_provider_error)?;
        if matches!(response.code, Some(404 | 40_363 | 40_404)) {
            return Err(BorrowingProviderError::TransactionNotFound);
        }
        ensure_business_success(response.code, response.success)?;
        let details = response
            .data
            .filter(|details| !details.is_empty())
            .ok_or(BorrowingProviderError::TransactionNotFound)?;
        BorrowingTransactionStatus::try_from(ProviderTransactionDetails(details))
    }
}

impl BinanceWeb3Client {
    async fn venus_investment_id(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<String, BorrowingProviderError> {
        let mut body = models::ListDefiInvestmentsRequest::new(
            models::list_defi_investments_request::InvestType::Earn,
        );
        body.defi_protocol_id = Some(VENUS_PROTOCOL_ID.to_owned());
        body.token_address_list = Some(vec![request.collateral_token.to_string()]);
        body.binance_chain_id = Some(BNB_CHAIN_ID.to_string());
        body.page = Some(1);
        body.size = Some(2);

        let response = list_defi_investments(
            &self.configuration,
            ListDefiInvestmentsParams {
                list_defi_investments_request: body,
            },
        )
        .await
        .map_err(map_provider_error)?;
        ensure_business_success(response.code, response.success)?;
        let investments = response.data.and_then(|page| page.list).unwrap_or_default();
        let investment = investments
            .into_iter()
            .next()
            .ok_or(BorrowingProviderError::Rejected)?;
        investment
            .investment_id
            .filter(|investment_id| !investment_id.is_empty())
            .ok_or(BorrowingProviderError::InvalidData)
    }
}

fn ensure_business_success(
    code: Option<i32>,
    success: Option<bool>,
) -> Result<(), BorrowingProviderError> {
    if is_business_success(code, success) {
        Ok(())
    } else {
        match code {
            Some(40_000..=49_999) if !matches!(code, Some(40_101..=40_104 | 42_900)) => {
                Err(BorrowingProviderError::Rejected)
            }
            _ => Err(BorrowingProviderError::Unavailable),
        }
    }
}

fn map_provider_error<T>(error: binance_web3_rwa::apis::Error<T>) -> BorrowingProviderError {
    match error {
        binance_web3_rwa::apis::Error::ResponseError(response)
            if response.status.is_client_error()
                && !matches!(
                    response.status,
                    reqwest::StatusCode::UNAUTHORIZED
                        | reqwest::StatusCode::FORBIDDEN
                        | reqwest::StatusCode::TOO_MANY_REQUESTS
                ) =>
        {
            BorrowingProviderError::Rejected
        }
        _ => BorrowingProviderError::Unavailable,
    }
}

fn map_transaction_provider_error<T>(
    error: binance_web3_rwa::apis::Error<T>,
) -> BorrowingProviderError {
    match error {
        binance_web3_rwa::apis::Error::ResponseError(response)
            if response.status == reqwest::StatusCode::NOT_FOUND =>
        {
            BorrowingProviderError::TransactionNotFound
        }
        error => map_provider_error(error),
    }
}
