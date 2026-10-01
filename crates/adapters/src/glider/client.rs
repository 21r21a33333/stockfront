use std::time::Duration;

use glider_api::{
    apis::{
        Error as GliderApiError,
        configuration::{ApiKey, Configuration},
        discovery_api::{DiscoveryStrategiesListParams, discovery_strategies_list},
        enrollment_api::{
            EnrollmentCreateParams, EnrollmentPrepareSignatureParams, enrollment_create,
            enrollment_prepare_signature,
        },
        portfolios_api::{
            PortfoliosGetOperationParams, PortfoliosGetParams, PortfoliosGetPositionsParams,
            PortfoliosListParams, PortfoliosPrepareLiquidateAllParams,
            PortfoliosPrepareWithdrawalParams, PortfoliosSubmitLiquidateAllParams,
            PortfoliosSubmitWithdrawalParams, portfolios_get, portfolios_get_operation,
            portfolios_get_positions, portfolios_list, portfolios_prepare_liquidate_all,
            portfolios_prepare_withdrawal, portfolios_submit_liquidate_all,
            portfolios_submit_withdrawal,
        },
    },
    models::{
        CreateEnrollRequest, CreateEnrollResponse, CreateEnrollSignatureRequest,
        CreateEnrollSignatureResponse, GetOperationResponse, GetPortfolioPositionsResponse,
        GetPortfolioResponse, LiquidateAllSignatureRequest, ListDiscoveryStrategiesResponse,
        ListPortfoliosResponse, WithdrawRequest, WithdrawResponse, WithdrawSignatureRequest,
        WithdrawSignatureResponse, create_enroll_request::AccountType as EnrollAccountType,
        create_enroll_signature_request::AccountType as PrepareAccountType,
    },
};
use reqwest_middleware::ClientBuilder;
use stockfront_core::managed_portfolios::{
    EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery, ListManagedPortfoliosQuery,
    ManagedPortfolioCollection, ManagedPortfolioProviderError, ManagedPortfolioSort,
    PrepareManagedPortfolioEnrollmentRequest,
};
use stockfront_core::wallet_portfolio::WalletAddress;
use thiserror::Error;
use url::Url;

pub(super) struct GliderClient {
    configuration: Configuration,
}

impl GliderClient {
    pub(super) fn new(
        base_url: &str,
        api_key: String,
        timeout: Duration,
    ) -> Result<Self, GliderConfigurationError> {
        let base_url = Url::parse(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|_| GliderConfigurationError::Unavailable)?;
        let configuration = Configuration {
            base_path: base_url.as_str().trim_end_matches('/').to_owned(),
            user_agent: Some(concat!("stockfront/", env!("CARGO_PKG_VERSION")).to_owned()),
            client: ClientBuilder::new(client).build(),
            api_key: Some(ApiKey {
                prefix: None,
                key: api_key,
            }),
            ..Configuration::default()
        };
        Ok(Self { configuration })
    }

    pub(super) async fn templates(
        &self,
        query: &ListManagedPortfolioTemplatesQuery,
    ) -> Result<ListDiscoveryStrategiesResponse, ManagedPortfolioProviderError> {
        let limit =
            i32::try_from(query.limit).map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        discovery_strategies_list(
            &self.configuration,
            DiscoveryStrategiesListParams {
                collection: match query.collection {
                    ManagedPortfolioCollection::Curated => "curated",
                    ManagedPortfolioCollection::TopPerforming => "top_performing",
                }
                .to_owned(),
                sort: query
                    .sort
                    .map(|sort| match sort {
                        ManagedPortfolioSort::Tvl => "tvl.desc",
                        ManagedPortfolioSort::Inflows7d => "inflows7d.desc",
                        ManagedPortfolioSort::Users => "users.desc",
                    })
                    .map(str::to_owned),
                limit: Some(limit),
                cursor: query.cursor.clone(),
            },
        )
        .await
        .map_err(|error| map_read_error(&error))
    }

    pub(super) async fn prepare_enrollment(
        &self,
        request: &PrepareManagedPortfolioEnrollmentRequest,
    ) -> Result<CreateEnrollSignatureResponse, ManagedPortfolioProviderError> {
        enrollment_prepare_signature(
            &self.configuration,
            EnrollmentPrepareSignatureParams {
                create_enroll_signature_request: CreateEnrollSignatureRequest::new(
                    evm_owner_account_id(request.wallet),
                    request.template_id.clone(),
                    vec![56],
                    PrepareAccountType::Ecdsa,
                ),
            },
        )
        .await
        .map_err(|error| map_enrollment_error(&error))
    }

    pub(super) async fn enroll(
        &self,
        request: &EnrollManagedPortfolioRequest,
    ) -> Result<CreateEnrollResponse, ManagedPortfolioProviderError> {
        let mut enrollment = CreateEnrollRequest::new(
            evm_owner_account_id(request.wallet),
            request.template_id.clone(),
            vec![56],
            EnrollAccountType::Ecdsa,
            request.authorization.account_index.clone(),
            request.authorization.agent_account_id.clone(),
            request.signature.clone(),
            request.authorization.flow_id.clone(),
        );
        enrollment.portfolio_name.clone_from(&request.name);

        enrollment_create(
            &self.configuration,
            EnrollmentCreateParams {
                create_enroll_request: enrollment,
            },
        )
        .await
        .map_err(|error| map_enrollment_error(&error))
    }

    pub(super) async fn portfolios(
        &self,
        wallet: WalletAddress,
        query: &ListManagedPortfoliosQuery,
    ) -> Result<ListPortfoliosResponse, ManagedPortfolioProviderError> {
        let limit =
            i32::try_from(query.limit).map_err(|_| ManagedPortfolioProviderError::InvalidData)?;
        portfolios_list(
            &self.configuration,
            PortfoliosListParams {
                owner_account_id: Some(evm_owner_account_id(wallet)),
                strategy_id: None,
                status: None,
                limit: Some(limit),
                cursor: query.cursor.clone(),
            },
        )
        .await
        .map_err(|error| map_read_error(&error))
    }

    pub(super) async fn positions(
        &self,
        portfolio_id: &str,
    ) -> Result<GetPortfolioPositionsResponse, ManagedPortfolioProviderError> {
        portfolios_get_positions(
            &self.configuration,
            PortfoliosGetPositionsParams {
                portfolio_id: portfolio_id.to_owned(),
            },
        )
        .await
        .map_err(|error| map_portfolio_error(&error))
    }

    pub(super) async fn portfolio(
        &self,
        portfolio_id: &str,
    ) -> Result<GetPortfolioResponse, ManagedPortfolioProviderError> {
        portfolios_get(
            &self.configuration,
            PortfoliosGetParams {
                portfolio_id: portfolio_id.to_owned(),
            },
        )
        .await
        .map_err(|error| map_portfolio_error(&error))
    }

    pub(super) async fn prepare_withdrawal(
        &self,
        portfolio_id: &str,
        request: WithdrawSignatureRequest,
    ) -> Result<WithdrawSignatureResponse, ManagedPortfolioProviderError> {
        portfolios_prepare_withdrawal(
            &self.configuration,
            PortfoliosPrepareWithdrawalParams {
                portfolio_id: portfolio_id.to_owned(),
                withdraw_signature_request: request,
            },
        )
        .await
        .map_err(|error| map_write_error(&error))
    }

    pub(super) async fn prepare_liquidation(
        &self,
        portfolio_id: &str,
        request: LiquidateAllSignatureRequest,
    ) -> Result<WithdrawSignatureResponse, ManagedPortfolioProviderError> {
        portfolios_prepare_liquidate_all(
            &self.configuration,
            PortfoliosPrepareLiquidateAllParams {
                portfolio_id: portfolio_id.to_owned(),
                liquidate_all_signature_request: request,
            },
        )
        .await
        .map_err(|error| map_write_error(&error))
    }

    pub(super) async fn submit_withdrawal(
        &self,
        portfolio_id: &str,
        request: WithdrawRequest,
        liquidate: bool,
    ) -> Result<WithdrawResponse, ManagedPortfolioProviderError> {
        if liquidate {
            portfolios_submit_liquidate_all(
                &self.configuration,
                PortfoliosSubmitLiquidateAllParams {
                    portfolio_id: portfolio_id.to_owned(),
                    withdraw_request: request,
                },
            )
            .await
            .map_err(|error| map_write_error(&error))
        } else {
            portfolios_submit_withdrawal(
                &self.configuration,
                PortfoliosSubmitWithdrawalParams {
                    portfolio_id: portfolio_id.to_owned(),
                    withdraw_request: request,
                },
            )
            .await
            .map_err(|error| map_write_error(&error))
        }
    }

    pub(super) async fn operation(
        &self,
        portfolio_id: &str,
        operation_id: &str,
    ) -> Result<GetOperationResponse, ManagedPortfolioProviderError> {
        portfolios_get_operation(
            &self.configuration,
            PortfoliosGetOperationParams {
                portfolio_id: portfolio_id.to_owned(),
                operation_id: operation_id.to_owned(),
            },
        )
        .await
        .map_err(|error| map_portfolio_error(&error))
    }
}

fn evm_owner_account_id(wallet: stockfront_core::wallet_portfolio::WalletAddress) -> String {
    format!("eip155:0:{}", wallet.as_address())
}

fn map_enrollment_error<E>(error: &GliderApiError<E>) -> ManagedPortfolioProviderError {
    match error {
        GliderApiError::Serde(_) | GliderApiError::SerdePathToError(_) => {
            ManagedPortfolioProviderError::InvalidData
        }
        GliderApiError::ResponseError(response) => match response.status.as_u16() {
            400 => ManagedPortfolioProviderError::Rejected,
            409 => ManagedPortfolioProviderError::Conflict,
            _ => ManagedPortfolioProviderError::Unavailable,
        },
        GliderApiError::Reqwest(_)
        | GliderApiError::ReqwestMiddleware(_)
        | GliderApiError::Io(_) => ManagedPortfolioProviderError::Unavailable,
    }
}

fn map_read_error<E>(error: &GliderApiError<E>) -> ManagedPortfolioProviderError {
    match error {
        GliderApiError::Serde(_) | GliderApiError::SerdePathToError(_) => {
            ManagedPortfolioProviderError::InvalidData
        }
        GliderApiError::Reqwest(_)
        | GliderApiError::ReqwestMiddleware(_)
        | GliderApiError::Io(_)
        | GliderApiError::ResponseError(_) => ManagedPortfolioProviderError::Unavailable,
    }
}

fn map_portfolio_error<E>(error: &GliderApiError<E>) -> ManagedPortfolioProviderError {
    match error {
        GliderApiError::ResponseError(response) if response.status.as_u16() == 404 => {
            ManagedPortfolioProviderError::NotFound
        }
        _ => map_read_error(error),
    }
}

fn map_write_error<E>(error: &GliderApiError<E>) -> ManagedPortfolioProviderError {
    match error {
        GliderApiError::Serde(_) | GliderApiError::SerdePathToError(_) => {
            ManagedPortfolioProviderError::InvalidData
        }
        GliderApiError::ResponseError(response) => match response.status.as_u16() {
            400 => ManagedPortfolioProviderError::Rejected,
            404 => ManagedPortfolioProviderError::NotFound,
            409 => ManagedPortfolioProviderError::Conflict,
            _ => ManagedPortfolioProviderError::Unavailable,
        },
        GliderApiError::Reqwest(_)
        | GliderApiError::ReqwestMiddleware(_)
        | GliderApiError::Io(_) => ManagedPortfolioProviderError::Unavailable,
    }
}

#[derive(Debug, Error)]
pub enum GliderConfigurationError {
    #[error("invalid Glider API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("the Glider API client is unavailable")]
    Unavailable,
}
