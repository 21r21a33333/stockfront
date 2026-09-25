use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use serde::Serialize;

use super::error::ApiError;

#[derive(Debug, Serialize)]
pub(super) enum Status {
    Ok,
    Error,
}

#[derive(Debug, Serialize)]
pub(super) struct Response<T> {
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Error>,
    #[serde(skip)]
    status_code: StatusCode,
}

impl<T> Response<T> {
    pub(super) fn ok(result: T) -> Self {
        Self {
            status: Status::Ok,
            result: Some(result),
            error: None,
            status_code: StatusCode::OK,
        }
    }
}

impl Response<()> {
    pub(super) fn error(code: &'static str, message: String, status_code: StatusCode) -> Self {
        Self {
            status: Status::Error,
            result: None,
            error: Some(Error { code, message }),
            status_code,
        }
    }
}

impl<T: Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> AxumResponse {
        let status_code = self.status_code;
        let mut response = Json(self).into_response();
        *response.status_mut() = status_code;
        response
    }
}

pub(super) type ApiResult<T> = Result<Response<T>, ApiError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct List<T> {
    items: Vec<T>,
}

impl<T> List<T> {
    pub(super) fn all(items: Vec<T>) -> Self {
        Self { items }
    }
}

#[derive(Debug, Serialize)]
struct Error {
    code: &'static str,
    message: String,
}
