use axum::{
    extract::{FromRequest, Json},
    http::{Request, StatusCode},
    response::Response,
};
use serde::de::DeserializeOwned;
use crate::structs::api_structs::ApiResponse;

pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let uri = req.uri().clone();
        let method = req.method().clone();

        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(ValidatedJson(value)),
            Err(rej) => Err(ApiResponse::err_and_log(
                "Invalid request body",
                StatusCode::BAD_REQUEST,
                rej,
                &[("endpoint", format!("{method} {uri}"))],
            )),
        }
    }
}