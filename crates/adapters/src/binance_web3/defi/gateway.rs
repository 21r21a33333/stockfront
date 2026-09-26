use alloy_primitives::{Address, B256};
use binance_web3_rwa::{
    apis::{
        de_fi_data_api::{ListDefiInvestmentsParams, list_defi_investments},
        de_fi_transaction_api::{
            BuildDefiDepositTransactionParams, BuildDefiRedeemTransactionParams,
            build_defi_deposit_transaction, build_defi_redeem_transaction,
        },
        wallet_api::{GetTransactionDetailByHashParams, get_transaction_detail_by_hash},
    },
    models,
};
use stockfront_core::{
    bstock_borrowing::{
        BorrowingProviderError, BorrowingTransactionStatus, PositionAmount, PrepareSupplyRequest,
        PrepareWithdrawalRequest, PreparedBorrowingAction,
    },
    bstock_market::BNB_CHAIN_ID,
};

use super::{
    super::client::BinanceWeb3Client,
    super::provider::{ensure_business_success, map_request_error},
    mapping::{ProviderPreparedAction, ProviderTransactionDetails},
};

const VENUS_PROTOCOL_ID: &str = "venus";

impl BinanceWeb3Client {
    pub(crate) async fn prepare_venus_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let investment_id = self.venus_investment_id(request.collateral_token).await?;
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
        .map_err(map_request_error)?;
        ensure_business_success(response.code, response.success)?;
        let plan = response.data.ok_or(BorrowingProviderError::InvalidData)?;
        PreparedBorrowingAction::try_from(ProviderPreparedAction(*plan))
    }

    pub(crate) async fn prepare_venus_withdrawal(
        &self,
        request: &PrepareWithdrawalRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let investment_id = self.venus_investment_id(request.collateral_token).await?;
        let mut body = models::BuildDefiRedeemRequest::new(
            request.wallet.as_address().to_string(),
            investment_id,
        );
        match &request.amount {
            PositionAmount::Exact { value } => {
                body.token = Some(Box::new(models::DefiTokenAmount::new(
                    request.collateral_token.to_string(),
                    value.to_string(),
                )));
            }
            PositionAmount::Full => body.ratio = Some("1".to_owned()),
        }
        body.simulate = Some(true);

        let response = build_defi_redeem_transaction(
            &self.configuration,
            BuildDefiRedeemTransactionParams {
                build_defi_redeem_request: body,
            },
        )
        .await
        .map_err(map_request_error)?;
        if response.code == Some(40_485) {
            return Err(BorrowingProviderError::InsufficientSuppliedBalance);
        }
        ensure_business_success(response.code, response.success)?;
        let plan = response.data.ok_or(BorrowingProviderError::InvalidData)?;
        PreparedBorrowingAction::try_from(ProviderPreparedAction(*plan))
    }

    pub(crate) async fn borrowing_transaction_status(
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
    async fn venus_investment_id(&self, token: Address) -> Result<String, BorrowingProviderError> {
        let mut body = models::ListDefiInvestmentsRequest::new(
            models::list_defi_investments_request::InvestType::Earn,
        );
        body.defi_protocol_id = Some(VENUS_PROTOCOL_ID.to_owned());
        body.token_address_list = Some(vec![token.to_string()]);
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
        .map_err(map_request_error)?;
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

fn map_transaction_provider_error<T>(
    error: binance_web3_rwa::apis::Error<T>,
) -> BorrowingProviderError {
    match error {
        binance_web3_rwa::apis::Error::ResponseError(response)
            if response.status == reqwest::StatusCode::NOT_FOUND =>
        {
            BorrowingProviderError::TransactionNotFound
        }
        error => map_request_error(error).into(),
    }
}
