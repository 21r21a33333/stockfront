use alloy_primitives::{Address, B256};
use binance_web3_rwa::{
    apis::{
        de_fi_data_api::{ListDefiInvestmentsParams, list_defi_investments},
        de_fi_transaction_api::{
            BuildDefiClaimTransactionParams, BuildDefiDepositTransactionParams,
            BuildDefiRedeemTransactionParams, build_defi_claim_transaction,
            build_defi_deposit_transaction, build_defi_redeem_transaction,
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
    carry::CarryWithdrawalAmount,
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
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
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
            .ok_or(DefiPlanError::InvalidData)
    }

    pub(super) async fn build_redeem_plan(
        &self,
        wallet: WalletAddress,
        investment_id: String,
        amount: &CarryWithdrawalAmount,
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
        let body = redeem_request(wallet, investment_id, amount);

        let response = build_defi_redeem_transaction(
            &self.configuration,
            BuildDefiRedeemTransactionParams {
                build_defi_redeem_request: body,
            },
        )
        .await
        .map_err(map_request_error)?;
        ensure_business_success(response.code, response.success)?;
        response
            .data
            .map(|plan| *plan)
            .ok_or(DefiPlanError::InvalidData)
    }

    pub(super) async fn build_redemption_claim_plan(
        &self,
        wallet: WalletAddress,
        investment_id: String,
        redemption_id: String,
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
        let body = redemption_claim_request(wallet, investment_id, redemption_id);

        let response = build_defi_claim_transaction(
            &self.configuration,
            BuildDefiClaimTransactionParams {
                build_defi_claim_request: body,
            },
        )
        .await
        .map_err(map_request_error)?;
        ensure_business_success(response.code, response.success)?;
        response
            .data
            .map(|plan| *plan)
            .ok_or(DefiPlanError::InvalidData)
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

fn redemption_claim_request(
    wallet: WalletAddress,
    investment_id: String,
    redemption_id: String,
) -> models::BuildDefiClaimRequest {
    let mut body = models::BuildDefiClaimRequest::new(
        wallet.as_address().to_string(),
        models::build_defi_claim_request::ClaimType::Redemption,
        investment_id,
        redemption_id,
    );
    body.simulate = Some(true);
    body
}

fn redeem_request(
    wallet: WalletAddress,
    investment_id: String,
    amount: &CarryWithdrawalAmount,
) -> models::BuildDefiRedeemRequest {
    let mut body =
        models::BuildDefiRedeemRequest::new(wallet.as_address().to_string(), investment_id);
    match amount {
        CarryWithdrawalAmount::Exact { token, value } => {
            body.token = Some(Box::new(models::DefiTokenAmount::new(
                token.to_string(),
                value.to_string(),
            )));
        }
        CarryWithdrawalAmount::Full => {
            body.ratio = Some("1".to_owned());
        }
    }
    body.simulate = Some(true);
    body
}

#[derive(Clone, Copy, Debug)]
pub(super) enum DefiPlanError {
    Request(BinanceRequestError),
    InvalidData,
}

impl From<BinanceRequestError> for DefiPlanError {
    fn from(error: BinanceRequestError) -> Self {
        Self::Request(error)
    }
}

impl From<DefiPlanError> for BorrowingProviderError {
    fn from(error: DefiPlanError) -> Self {
        match error {
            DefiPlanError::Request(error) => error.into(),
            DefiPlanError::InvalidData => Self::InvalidData,
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

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use rust_decimal::Decimal;
    use stockfront_core::carry::CarryWithdrawalAmount;

    use super::*;

    #[test]
    fn exact_redeem_request_uses_token_amount_without_ratio() {
        let token = Address::with_last_byte(3);
        let request = redeem_request(
            wallet(),
            "investment".to_owned(),
            &CarryWithdrawalAmount::Exact {
                token,
                value: Decimal::new(25, 1),
            },
        );

        assert_eq!(request.ratio, None);
        let amount = request
            .token
            .unwrap_or_else(|| panic!("exact request omitted token amount"));
        assert_eq!(amount.token_address, token.to_string());
        assert_eq!(amount.amount, "2.5");
        assert_eq!(request.simulate, Some(true));
    }

    #[test]
    fn full_redeem_request_uses_full_ratio_without_token_amount() {
        let request = redeem_request(
            wallet(),
            "investment".to_owned(),
            &CarryWithdrawalAmount::Full,
        );

        assert_eq!(request.token, None);
        assert_eq!(request.ratio.as_deref(), Some("1"));
        assert_eq!(request.simulate, Some(true));
    }

    #[test]
    fn redemption_claim_request_uses_provider_ids_and_simulation() {
        let wallet = wallet();
        let request =
            redemption_claim_request(wallet, "investment".to_owned(), "redemption".to_owned());

        assert_eq!(request.address, wallet.as_address().to_string());
        assert_eq!(
            request.claim_type,
            models::build_defi_claim_request::ClaimType::Redemption
        );
        assert_eq!(request.investment_id, "investment");
        assert_eq!(request.redemption_id, "redemption");
        assert_eq!(request.simulate, Some(true));
    }

    fn wallet() -> WalletAddress {
        WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"))
    }
}
