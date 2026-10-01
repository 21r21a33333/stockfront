use glider_api::models::{
    LiquidateAllSignatureRequest, Operation, WithdrawData, WithdrawEip712Field,
    WithdrawEip712MessageBody, WithdrawEip712TypedData, WithdrawMessage,
    WithdrawMessageAssetsInner, WithdrawRequest, WithdrawSignatureData, WithdrawSignatureRequest,
    WithdrawalAsset,
    operation::State,
    withdraw_eip712_domain::{Name, Version},
};
use stockfront_core::{
    bstock_market::BNB_CHAIN_ID,
    managed_portfolios::{
        ManagedPortfolioOperation, ManagedPortfolioOperationState, ManagedPortfolioProviderError,
        ManagedPortfolioWithdrawalEip712Domain, ManagedPortfolioWithdrawalEip712Field,
        ManagedPortfolioWithdrawalEip712Types, ManagedPortfolioWithdrawalMessage,
        ManagedPortfolioWithdrawalMessageAsset, ManagedPortfolioWithdrawalTypedData,
        PrepareManagedPortfolioWithdrawalRequest, PreparedManagedPortfolioWithdrawal,
        SubmitManagedPortfolioWithdrawalRequest, SubmittedManagedPortfolioWithdrawal,
    },
    wallet_portfolio::WalletAddress,
};

use super::{GliderAdapter, model::parse_timestamp};

const BNB_USDT_ASSET_ID: &str = "eip155:56/erc20:0x55d398326f99059ff775485246999027b3197955";

struct BnbPreparedWithdrawal(PreparedManagedPortfolioWithdrawal);

struct GliderWithdrawalRequest(WithdrawRequest);

struct BnbSubmittedWithdrawal(SubmittedManagedPortfolioWithdrawal);

struct BnbWithdrawalOperation(ManagedPortfolioOperation);

impl GliderAdapter {
    pub(super) async fn prepare_withdrawal_action(
        &self,
        portfolio_id: &str,
        request: &PrepareManagedPortfolioWithdrawalRequest,
    ) -> Result<PreparedManagedPortfolioWithdrawal, ManagedPortfolioProviderError> {
        let response = match request {
            PrepareManagedPortfolioWithdrawalRequest::InKind { recipient, assets } => {
                self.client
                    .prepare_withdrawal(
                        portfolio_id,
                        WithdrawSignatureRequest::new(
                            bnb_account_id(*recipient),
                            assets
                                .iter()
                                .map(|asset| {
                                    WithdrawalAsset::new(
                                        bnb_asset_id(asset.token),
                                        asset.raw_amount.to_string(),
                                    )
                                })
                                .collect(),
                        ),
                    )
                    .await?
            }
            PrepareManagedPortfolioWithdrawalRequest::FullToUsdt { recipient } => {
                let mut request = LiquidateAllSignatureRequest::new(bnb_account_id(*recipient));
                request.settlement_asset_id = Some(BNB_USDT_ASSET_ID.to_owned());
                self.client
                    .prepare_liquidation(portfolio_id, request)
                    .await?
            }
        };

        BnbPreparedWithdrawal::try_from(*response.data).map(|value| value.0)
    }

    pub(super) async fn submit_withdrawal_action(
        &self,
        portfolio_id: &str,
        request: &SubmitManagedPortfolioWithdrawalRequest,
    ) -> Result<SubmittedManagedPortfolioWithdrawal, ManagedPortfolioProviderError> {
        let liquidate = request.message.liquidate == Some(true);
        let response = self
            .client
            .submit_withdrawal(
                portfolio_id,
                GliderWithdrawalRequest::from(request).0,
                liquidate,
            )
            .await?;
        BnbSubmittedWithdrawal::try_from(*response.data).map(|value| value.0)
    }

    pub(super) async fn withdrawal_operation(
        &self,
        portfolio_id: &str,
        operation_id: &str,
    ) -> Result<ManagedPortfolioOperation, ManagedPortfolioProviderError> {
        BnbWithdrawalOperation::try_from(
            *self
                .client
                .operation(portfolio_id, operation_id)
                .await?
                .data,
        )
        .map(|value| value.0)
    }
}

impl TryFrom<WithdrawSignatureData> for BnbPreparedWithdrawal {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: WithdrawSignatureData) -> Result<Self, Self::Error> {
        let typed_data = value
            .typed_data
            .ok_or(ManagedPortfolioProviderError::InvalidData)?;
        Ok(Self(PreparedManagedPortfolioWithdrawal {
            authorization_id: value.authorization_id,
            expires_at_ms: parse_timestamp(&value.expires_at)?,
            typed_data: map_typed_data(*typed_data),
        }))
    }
}

fn map_typed_data(value: WithdrawEip712TypedData) -> ManagedPortfolioWithdrawalTypedData {
    ManagedPortfolioWithdrawalTypedData {
        primary_type: "Withdrawal".to_owned(),
        domain: map_domain(*value.domain),
        types: ManagedPortfolioWithdrawalEip712Types {
            withdrawal: value.types.withdrawal.iter().map(map_field).collect(),
            withdrawal_asset: value.types.withdrawal_asset.iter().map(map_field).collect(),
        },
        message: map_message(*value.message),
    }
}

fn map_domain(
    value: glider_api::models::WithdrawEip712Domain,
) -> ManagedPortfolioWithdrawalEip712Domain {
    ManagedPortfolioWithdrawalEip712Domain {
        name: match value.name {
            Name::GliderWithdrawalAuthorization => "Glider Withdrawal Authorization".to_owned(),
        },
        version: match value.version {
            Version::Variant1 => "1".to_owned(),
            Version::Variant2 => "2".to_owned(),
        },
        chain_id: value.chain_id,
        verifying_contract: value.verifying_contract,
    }
}

fn map_field(value: &WithdrawEip712Field) -> ManagedPortfolioWithdrawalEip712Field {
    ManagedPortfolioWithdrawalEip712Field {
        name: value.name.clone(),
        field_type: value.r#type.clone(),
    }
}

fn map_message(value: WithdrawEip712MessageBody) -> ManagedPortfolioWithdrawalMessage {
    ManagedPortfolioWithdrawalMessage {
        portfolio_id: value.portfolio_id,
        recipient_account_id: value.recipient_account_id,
        assets: value
            .assets
            .into_iter()
            .map(|asset| ManagedPortfolioWithdrawalMessageAsset {
                asset_id: asset.asset_id,
                amount_raw: asset.amount_raw,
            })
            .collect(),
        nonce: value.nonce,
        expires_at: value.expires_at,
        liquidate: value.liquidate,
        settlement_asset_id: value.settlement_asset_id,
    }
}

impl From<&SubmitManagedPortfolioWithdrawalRequest> for GliderWithdrawalRequest {
    fn from(value: &SubmitManagedPortfolioWithdrawalRequest) -> Self {
        Self(WithdrawRequest::new(
            withdraw_message(&value.message),
            value.signature.clone(),
        ))
    }
}

fn withdraw_message(value: &ManagedPortfolioWithdrawalMessage) -> WithdrawMessage {
    let mut message = WithdrawMessage::new(
        value.portfolio_id.clone(),
        value.recipient_account_id.clone(),
        value
            .assets
            .iter()
            .map(|asset| {
                WithdrawMessageAssetsInner::new(asset.asset_id.clone(), asset.amount_raw.clone())
            })
            .collect(),
        value.nonce.clone(),
        value.expires_at,
    );
    message.liquidate = value.liquidate;
    message
        .settlement_asset_id
        .clone_from(&value.settlement_asset_id);
    message
}

impl TryFrom<WithdrawData> for BnbSubmittedWithdrawal {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: WithdrawData) -> Result<Self, Self::Error> {
        Ok(Self(SubmittedManagedPortfolioWithdrawal {
            operation_id: value.operation_id,
            submitted_at_ms: parse_timestamp(&value.submitted_at)?,
        }))
    }
}

impl TryFrom<Operation> for BnbWithdrawalOperation {
    type Error = ManagedPortfolioProviderError;

    fn try_from(value: Operation) -> Result<Self, Self::Error> {
        Ok(Self(ManagedPortfolioOperation {
            operation_id: value.operation_id,
            state: match value.state {
                State::Accepted => ManagedPortfolioOperationState::Accepted,
                State::Running => ManagedPortfolioOperationState::Running,
                State::Retrying => ManagedPortfolioOperationState::Retrying,
                State::AwaitingUser => ManagedPortfolioOperationState::AwaitingUser,
                State::Completed => ManagedPortfolioOperationState::Completed,
                State::Failed => ManagedPortfolioOperationState::Failed,
                State::Cancelled => ManagedPortfolioOperationState::Cancelled,
            },
            updated_at_ms: parse_timestamp(&value.updated_at)?,
            finished_at_ms: value
                .finished_at
                .as_deref()
                .map(parse_timestamp)
                .transpose()?,
            error: value.error,
        }))
    }
}

fn bnb_account_id(wallet: WalletAddress) -> String {
    format!("eip155:{BNB_CHAIN_ID}:{}", wallet.as_address())
}

fn bnb_asset_id(token: alloy_primitives::Address) -> String {
    format!("eip155:{BNB_CHAIN_ID}/erc20:{token}")
}

#[cfg(test)]
mod tests {
    use glider_api::models::{Operation, WithdrawSignatureData};

    use super::*;

    #[test]
    fn maps_evm_withdrawal_authorization_without_changing_signed_message() {
        let data = serde_json::from_value::<WithdrawSignatureData>(serde_json::json!({
            "authorizationId": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "expiresAt": "2026-10-01T12:10:00.000Z",
            "typedData": {
                "primaryType": "Withdrawal",
                "domain": {
                    "name": "Glider Withdrawal Authorization",
                    "version": "1",
                    "chainId": 56,
                    "verifyingContract": "0xfe876c86bE8C81583169AE2Dd7F7F3D28643931a"
                },
                "types": {
                    "Withdrawal": [
                        { "name": "portfolioId", "type": "string" },
                        { "name": "assets", "type": "WithdrawalAsset[]" }
                    ],
                    "WithdrawalAsset": [
                        { "name": "assetId", "type": "string" },
                        { "name": "amountRaw", "type": "string" }
                    ]
                },
                "message": {
                    "portfolioId": "tvqolmgl",
                    "recipientAccountId": "eip155:56:0x4b393bEc5DeE11799F698440AB19c0FBfe15d74c",
                    "assets": [{
                        "assetId": "eip155:56/erc20:0x55d398326f99059fF775485246999027B3197955",
                        "amountRaw": "1000000000000000000"
                    }],
                    "nonce": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "expiresAt": 1_790_856_600
                }
            }
        }))
        .unwrap_or_else(|error| panic!("valid generated withdrawal rejected: {error}"));

        let prepared = BnbPreparedWithdrawal::try_from(data)
            .unwrap_or_else(|error| panic!("valid BNB withdrawal rejected: {error}"))
            .0;

        assert_eq!(prepared.typed_data.primary_type, "Withdrawal");
        assert_eq!(prepared.typed_data.domain.chain_id, 56);
        assert_eq!(prepared.typed_data.domain.version, "1");
        assert_eq!(prepared.typed_data.message.portfolio_id, "tvqolmgl");
        assert_eq!(prepared.typed_data.message.assets.len(), 1);
        assert_eq!(
            prepared.typed_data.message.assets[0].amount_raw,
            "1000000000000000000"
        );
        assert_eq!(prepared.typed_data.message.liquidate, None);
        assert_eq!(prepared.expires_at_ms, 1_790_856_600_000);

        let provider_request =
            GliderWithdrawalRequest::from(&SubmitManagedPortfolioWithdrawalRequest {
                message: prepared.typed_data.message.clone(),
                signature: "0xsignature".to_owned(),
            })
            .0;
        assert_eq!(provider_request.signature, "0xsignature");
        assert_eq!(provider_request.message.portfolio_id, "tvqolmgl");
        assert_eq!(
            provider_request.message.assets[0].amount_raw,
            "1000000000000000000"
        );
        assert_eq!(provider_request.message.liquidate, None);
    }

    #[test]
    fn maps_terminal_withdrawal_operation() {
        let operation = serde_json::from_value::<Operation>(serde_json::json!({
            "operationId": "operation-1",
            "portfolioId": "tvqolmgl",
            "kind": "withdraw",
            "state": "completed",
            "createdAt": "2026-10-01T12:00:00.000Z",
            "updatedAt": "2026-10-01T12:01:00.000Z",
            "finishedAt": "2026-10-01T12:01:00.000Z",
            "error": null
        }))
        .unwrap_or_else(|error| panic!("valid generated operation rejected: {error}"));

        let operation = BnbWithdrawalOperation::try_from(operation)
            .unwrap_or_else(|error| panic!("valid BNB operation rejected: {error}"))
            .0;

        assert_eq!(operation.operation_id, "operation-1");
        assert_eq!(operation.state, ManagedPortfolioOperationState::Completed);
        assert_eq!(operation.updated_at_ms, 1_790_856_060_000);
        assert_eq!(operation.finished_at_ms, Some(1_790_856_060_000));
        assert_eq!(operation.error, None);
    }

    #[test]
    fn maps_full_liquidation_boolean_authorization() {
        let data = serde_json::from_value::<WithdrawSignatureData>(serde_json::json!({
            "authorizationId": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "expiresAt": "2026-10-01T12:10:00.000Z",
            "typedData": {
                "primaryType": "Withdrawal",
                "domain": {
                    "name": "Glider Withdrawal Authorization",
                    "version": "2",
                    "chainId": 56,
                    "verifyingContract": "0xfe876c86bE8C81583169AE2Dd7F7F3D28643931a"
                },
                "types": {
                    "Withdrawal": [{ "name": "liquidate", "type": "bool" }],
                    "WithdrawalAsset": [
                        { "name": "assetId", "type": "string" },
                        { "name": "amountRaw", "type": "string" }
                    ]
                },
                "message": {
                    "portfolioId": "tvqolmgl",
                    "recipientAccountId": "eip155:56:0x4b393bEc5DeE11799F698440AB19c0FBfe15d74c",
                    "assets": [{
                        "assetId": "eip155:56/erc20:0x55d398326f99059fF775485246999027B3197955",
                        "amountRaw": "2000000000000000000"
                    }],
                    "nonce": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "expiresAt": 1_790_856_600,
                    "liquidate": true,
                    "settlementAssetId": "eip155:56/erc20:0x55d398326f99059fF775485246999027B3197955"
                }
            }
        }))
        .unwrap_or_else(|error| panic!("valid liquidation authorization rejected: {error}"));

        let prepared = BnbPreparedWithdrawal::try_from(data)
            .unwrap_or_else(|error| panic!("valid BNB liquidation rejected: {error}"))
            .0;

        assert_eq!(prepared.typed_data.domain.version, "2");
        assert_eq!(prepared.typed_data.message.liquidate, Some(true));
        assert_eq!(
            prepared.typed_data.message.settlement_asset_id.as_deref(),
            Some("eip155:56/erc20:0x55d398326f99059fF775485246999027B3197955")
        );
    }
}
