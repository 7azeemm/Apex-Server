use std::fmt::Display;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(flatten)]
    payload: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(payload: T) -> Response {
        (
            StatusCode::OK,
            Json(ApiResponse {
                error: None,
                payload,
            }),
        ).into_response()
    }
}

impl ApiResponse<()> {
    fn log(message: String, error: impl Display, context: &[(&str, String)]) {
        if error.to_string().is_empty() {
            error!(
                context = %serde_json::to_string(&context).unwrap_or_default(),
                "{}", message
            )
        } else {
            error!(
                %error,
                context = %serde_json::to_string(&context).unwrap_or_default(),
                "{}", message
            )
        }
    }

    pub fn internal_err(msg: impl Into<String> + Display, error: impl Display, context: &[(&str, String)]) -> Response {
        Self::log(msg.to_string(), error, context);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                error: Some(msg.into()),
                payload: (),
            }),
        ).into_response()
    }

    pub fn err(msg: impl Into<String> + Display, status: StatusCode) -> Response {
        (
            status,
            Json(ApiResponse {
                error: Some(msg.into()),
                payload: (),
            }),
        ).into_response()
    }

    pub fn err_and_log(msg: impl Into<String> + Display, status: StatusCode, error: impl Display, context: &[(&str, String)]) -> Response {
        Self::log(msg.to_string(), error, context);
        Self::err(msg, status)
    }
}