use axum::response::{IntoResponse, Response};
use maud::html;
use reqwest::StatusCode;

use crate::{components, icons};

pub enum ErrorResponse {
    InternalServerError,
    StatusCode(StatusCode),
    Alert,
    Toast,
}

pub trait MapErrorResponse<T> {
    fn map_err_response(self, mapper: ErrorResponse) -> Result<T, Response>;
}

impl<T, E: ToString> MapErrorResponse<T> for Result<T, E> {
    fn map_err_response(self, mapper: ErrorResponse) -> Result<T, Response> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(mapper.transform(err)),
        }
    }
}

impl ErrorResponse {
    pub fn transform<E: ToString>(&self, err: E) -> Response {
        match self {
            Self::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            Self::StatusCode(code) => (*code, err.to_string()).into_response(),
            Self::Alert => {
                html! { ."alert"."alert-error" {(icons::error()) span {(err.to_string())}} }
                    .into_response()
            }
            Self::Toast => components::ToastAlert::Error(&err.to_string()).into_response(),
        }
    }
}
