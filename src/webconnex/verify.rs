use askama_axum::IntoResponse;
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Clone)]
pub struct VerifySigState {
    pub hmac_secret: String,
}

pub async fn ver_sig(
    State(state): State<VerifySigState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let (parts, body) = req.into_parts();

    let mut hmac = Hmac::<Sha256>::new_from_slice(state.hmac_secret.as_bytes())
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    hmac.update(body_bytes.as_ref());

    let signature = parts
        .headers
        .get("X-Webconnex-Signature")
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "X-Webconnex-Signature header missing",
            )
                .into_response()
        })?
        .as_bytes();

    hmac.verify_slice(hex::decode(signature).unwrap().as_slice())
        .map_err(|err| (StatusCode::UNAUTHORIZED, err.to_string()).into_response())?;

    Ok(next
        .run(Request::from_parts(parts, Body::from(body_bytes)))
        .await)
}
