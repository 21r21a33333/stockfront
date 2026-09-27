use alloy_primitives::{Address, B256};
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
        BorrowingProviderError, BorrowingTransactionStatus, PrepareSupplyRequest,
        PreparedBorrowingAction,
    },
    bstock_market::BNB_CHAIN_ID,
    wallet_portfolio::WalletAddress,
};

use super::{
    super::client::BinanceWeb3Client,
    super::provider::{BinanceRequestError, ensure_business_success, map_request_error},
    mapping::{ProviderPreparedAction, ProviderTransactionDetails},
};

const VENUS_PROTOCOL_ID: &str = "venus";

impl BinanceWeb3Client {
    pub(crate) async fn prepare_venus_supply(
        &self,
        request: &PrepareSupplyRequest,
    ) -> Result<PreparedBorrowingAction, BorrowingProviderError> {
        let investment_id = self.venus_investment_id(request.collateral_token).await?;
        let plan = self
            .build_deposit_plan(
                request.wallet,
                investment_id,
                request.collateral_token,
                request.amount,
            )
            .await
            .map_err(BorrowingProviderError::from)?;
        PreparedBorrowingAction::try_from(ProviderPreparedAction(plan))
    }

    pub(super) async fn build_deposit_plan(
        &self,
        wallet: WalletAddress,
        investment_id: String,
        token: Address,
        amount: rust_decimal::Decimal,
    ) -> Result<models::DefiTransactionPlan, DepositPlanError> {
        let token = models::DefiTokenAmount::new(token.to_string(), amount.to_string());
        let mut body = models::BuildDefiDepositRequest::new(
            wallet.as_address().to_string(),
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
        response
            .data
            .map(|plan| *plan)
            .ok_or(DepositPlanError::InvalidData)
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

#[derive(Clone, Copy, Debug)]
pub(super) enum DepositPlanError {
    Request(BinanceRequestError),
    InvalidData,
}

impl From<BinanceRequestError> for DepositPlanError {
    fn from(error: BinanceRequestError) -> Self {
        Self::Request(error)
    }
}

impl From<DepositPlanError> for BorrowingProviderError {
    fn from(error: DepositPlanError) -> Self {
        match error {
            DepositPlanError::Request(error) => error.into(),
            DepositPlanError::InvalidData => Self::InvalidData,
        }
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
