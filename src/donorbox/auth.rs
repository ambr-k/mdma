use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use sha2::Sha256;

use crate::err_responses::{ErrorResponse, MapErrorResponse};

pub async fn ver_sig(
    State(hmac_secret): State<String>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let (parts, body) = req.into_parts();
    let mut hmac = Hmac::<Sha256>::new_from_slice(hmac_secret.as_bytes())
        .map_err_response(ErrorResponse::InternalServerError)?;

    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;

    let (timestamp, signature) = parts
        .headers
        .get("Donorbox-Signature")
        .ok_or("Donorbox-Signature header missing")
        .map_err_response(ErrorResponse::StatusCode(StatusCode::UNAUTHORIZED))?
        .to_str()
        .map_err_response(ErrorResponse::StatusCode(StatusCode::BAD_REQUEST))?
        .split_once(',')
        .ok_or("Donorbox-Signature header invalid")
        .map_err_response(ErrorResponse::StatusCode(StatusCode::BAD_REQUEST))?;

    hmac.update(timestamp.as_bytes());
    hmac.update(b".");
    hmac.update(body_bytes.as_ref());

    hmac.verify_slice(hex::decode(signature).unwrap().as_slice())
        .map_err_response(ErrorResponse::StatusCode(StatusCode::UNAUTHORIZED))?;

    Ok(next
        .run(Request::from_parts(parts, Body::from(body_bytes)))
        .await)
}
