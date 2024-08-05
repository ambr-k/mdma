use axum::response::{IntoResponse, Response};
use maud::html;
use reqwest::StatusCode;

use crate::{components, icons};

pub enum ErrorResponse<'a> {
    InternalServerError,
    StatusCode(StatusCode),
    Alert,
    AlertWithPrelude(&'a str),
    Toast,
}

pub trait MapErrorResponse<T> {
    fn map_err_response(self, mapper: ErrorResponse) -> Result<T, Response>;
}

impl<T, E: ToString> MapErrorResponse<T> for Result<T, E> {
    fn map_err_response(self, mapper: ErrorResponse) -> Result<T, Response> {
        self.map_err(|err| mapper.transform(err))
    }
}

impl ErrorResponse<'_> {
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
            Self::AlertWithPrelude(prelude) => {
                html! { ."alert"."alert-error" {(icons::error()) span {(prelude)": "(err.to_string())}} }
                    .into_response()
            }
            Self::Toast => components::ToastAlert::Error(&err.to_string()).into_response(),
        }
    }
}
