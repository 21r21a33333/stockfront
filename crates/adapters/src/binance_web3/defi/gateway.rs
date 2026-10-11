use alloy_primitives::{Address, B256};
use binance_web3_rwa::{
    apis::{
        de_fi_transaction_api::{
            BuildDefiClaimTransactionParams, BuildDefiDepositTransactionParams,
            BuildDefiRedeemTransactionParams, BuildLpAddTransactionParams,
            BuildLpRemoveTransactionParams, CalculateLpAddPairedAmountsParams,
            build_defi_claim_transaction, build_defi_deposit_transaction,
            build_defi_redeem_transaction, build_lp_add_transaction, build_lp_remove_transaction,
            calculate_lp_add_paired_amounts,
        },
        wallet_api::{GetTransactionDetailByHashParams, get_transaction_detail_by_hash},
    },
    models,
};
use stockfront_core::{
    bstock_borrowing::{BorrowingProviderError, BorrowingTransactionStatus},
    bstock_market::BNB_CHAIN_ID,
    liquidity::{LiquidityWithdrawalCommand, PrepareLiquidityClaimRequest},
    wallet_portfolio::WalletAddress,
};

use super::{
    super::client::BinanceWeb3Client,
    super::provider::{BinanceRequestError, ensure_business_success, map_request_error},
    mapping::ProviderTransactionDetails,
};

pub(super) struct LpEntryPlan {
    pub(super) token_amounts: Vec<models::DefiTokenAmount>,
    pub(super) transactions: models::DefiTransactionPlan,
}

impl BinanceWeb3Client {
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
        token: Address,
        amount: rust_decimal::Decimal,
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
        let body = redeem_request(wallet, investment_id, token, amount);

        let response = build_defi_redeem_transaction(
            &self.configuration,
            BuildDefiRedeemTransactionParams {
                build_defi_redeem_request: body,
            },
        )
        .await
        .map_err(map_request_error)?;
        if let Err(error) = ensure_business_success(response.code, response.success) {
            // Binance explains a refusal in `msg`. Without it every refusal
            // looks the same at the port, and the user is told only that the
            // provider said no.
            tracing::warn!(
                code = response.code,
                message = response.msg.as_deref().unwrap_or("none"),
                "the earn provider refused to build a redeem transaction"
            );
            return Err(error.into());
        }
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

    pub(super) async fn build_lp_fee_claim_plan(
        &self,
        request: &PrepareLiquidityClaimRequest,
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
        let body = lp_fee_claim_request(request);

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

    pub(super) async fn build_lp_entry_plan(
        &self,
        wallet: WalletAddress,
        investment_id: String,
        input_token: Address,
        input_amount: rust_decimal::Decimal,
        price_range_percent: rust_decimal::Decimal,
        slippage_bps: u32,
    ) -> Result<LpEntryPlan, DefiPlanError> {
        let input_token =
            models::DefiTokenAmount::new(input_token.to_string(), input_amount.to_string());
        let mut calculation = models::CalculateLpAddPairedAmountsRequest::new(
            wallet.as_address().to_string(),
            investment_id.clone(),
            input_token.clone(),
        );
        calculation.price_range = Some(price_range_percent.to_string());
        let response = calculate_lp_add_paired_amounts(
            &self.configuration,
            CalculateLpAddPairedAmountsParams {
                calculate_lp_add_paired_amounts_request: calculation,
            },
        )
        .await
        .map_err(map_request_error)?;
        ensure_business_success(response.code, response.success)?;
        let amounts = response.data.ok_or(DefiPlanError::InvalidData)?;
        let token_amounts = amounts.outputs;
        let mut request = models::BuildLpAddRequest::new(
            wallet.as_address().to_string(),
            investment_id,
            token_amounts.clone(),
        );
        request.tick_lower = Some(amounts.pool_info.tick_lower.clone());
        request.tick_upper = Some(amounts.pool_info.tick_upper.clone());
        request.slippage_bps = Some(slippage_bps.to_string());
        request.simulate = Some(true);
        let response = build_lp_add_transaction(
            &self.configuration,
            BuildLpAddTransactionParams {
                build_lp_add_request: request,
            },
        )
        .await
        .map_err(map_request_error)?;
        ensure_business_success(response.code, response.success)?;
        let transactions = response.data.ok_or(DefiPlanError::InvalidData)?;

        Ok(LpEntryPlan {
            token_amounts,
            transactions: *transactions,
        })
    }

    pub(super) async fn build_lp_withdrawal_plan(
        &self,
        command: &LiquidityWithdrawalCommand,
    ) -> Result<models::DefiTransactionPlan, DefiPlanError> {
        let body = lp_remove_request(command);
        let response = build_lp_remove_transaction(
            &self.configuration,
            BuildLpRemoveTransactionParams {
                build_lp_remove_request: body,
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
}

fn lp_remove_request(command: &LiquidityWithdrawalCommand) -> models::BuildLpRemoveRequest {
    let mut body = models::BuildLpRemoveRequest::new(
        command.wallet.as_address().to_string(),
        command.investment_id.clone(),
        command.nft_id.clone(),
        command.ratio.to_string(),
    );
    body.slippage_bps = Some(command.slippage_bps.to_string());
    body.simulate = Some(true);
    body
}

fn redemption_claim_request(
    wallet: WalletAddress,
    investment_id: String,
    redemption_id: String,
) -> models::BuildDefiClaimRequest {
    let mut body = models::BuildDefiClaimRequest::new(
        wallet.as_address().to_string(),
        models::build_defi_claim_request::ClaimType::Redemption,
    );
    body.investment_id = Some(investment_id);
    body.redemption_id = Some(redemption_id);
    body.simulate = Some(true);
    body
}

fn lp_fee_claim_request(request: &PrepareLiquidityClaimRequest) -> models::BuildDefiClaimRequest {
    let claim_type = match request.claim_type {
        stockfront_core::liquidity::LiquidityClaimType::LpFee => {
            models::build_defi_claim_request::ClaimType::LpFee
        }
    };
    let mut body =
        models::BuildDefiClaimRequest::new(request.wallet.as_address().to_string(), claim_type);
    body.investment_id = Some(request.investment_id.clone());
    body.nft_id = Some(request.nft_id.clone());
    body.simulate = Some(true);
    body
}

fn redeem_request(
    wallet: WalletAddress,
    investment_id: String,
    token: Address,
    amount: rust_decimal::Decimal,
) -> models::BuildDefiRedeemRequest {
    let mut body =
        models::BuildDefiRedeemRequest::new(wallet.as_address().to_string(), investment_id);
    body.token = Some(Box::new(models::DefiTokenAmount::new(
        token.to_string(),
        amount.to_string(),
    )));
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

    use super::*;

    #[test]
    fn redeem_request_uses_exact_token_amount() {
        let token = Address::with_last_byte(3);
        let request = redeem_request(
            wallet(),
            "investment".to_owned(),
            token,
            Decimal::new(25, 1),
        );

        let amount = request
            .token
            .unwrap_or_else(|| panic!("exact request omitted token amount"));
        assert_eq!(amount.token_address, token.to_string());
        assert_eq!(amount.amount, "2.5");
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
        assert_eq!(request.investment_id.as_deref(), Some("investment"));
        assert_eq!(request.redemption_id.as_deref(), Some("redemption"));
        assert_eq!(request.simulate, Some(true));
    }

    #[test]
    fn lp_fee_claim_uses_investment_and_nft_ids() {
        let request = lp_fee_claim_request(&PrepareLiquidityClaimRequest {
            wallet: wallet(),
            investment_id: "investment".to_owned(),
            claim_type: stockfront_core::liquidity::LiquidityClaimType::LpFee,
            nft_id: "#7582092".to_owned(),
        });

        assert_eq!(
            request.claim_type,
            models::build_defi_claim_request::ClaimType::LpFee
        );
        assert_eq!(request.investment_id.as_deref(), Some("investment"));
        assert_eq!(request.nft_id.as_deref(), Some("#7582092"));
        assert_eq!(request.simulate, Some(true));
    }

    #[test]
    fn lp_remove_uses_position_ratio_policy_and_simulation() {
        let request = lp_remove_request(&LiquidityWithdrawalCommand {
            wallet: wallet(),
            investment_id: "investment".to_owned(),
            nft_id: "#7582092".to_owned(),
            ratio: Decimal::new(25, 2),
            slippage_bps: 100,
        });

        assert_eq!(request.investment_id, "investment");
        assert_eq!(request.nft_id, "#7582092");
        assert_eq!(request.ratio, "0.25");
        assert_eq!(request.slippage_bps.as_deref(), Some("100"));
        assert_eq!(request.simulate, Some(true));
    }

    fn wallet() -> WalletAddress {
        WalletAddress::try_from(Address::with_last_byte(9))
            .unwrap_or_else(|error| panic!("valid wallet rejected: {error}"))
    }
}
