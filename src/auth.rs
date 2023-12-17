use askama_axum::IntoResponse;
use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderValue, StatusCode},
    response::{Redirect, Response},
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::AppState;

#[derive(Serialize, Deserialize)]
struct AccountRecord {
    id: i32,
    email: String,
    is_admin: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Jwt {
    account: AccountRecord,
    exp: OffsetDateTime,
}

pub fn oauth_client(client_id: String, client_secret: String, redirect_url: String) -> BasicClient {
    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

pub async fn signin_redirect(State(state): State<crate::AppState>) -> Redirect {
    let url = state
        .google_oauth
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .url()
        .0;

    Redirect::to(url.as_str())
}

#[derive(Deserialize)]
pub struct OauthCallbackQuery {
    code: String,
}

#[derive(Deserialize)]
struct GoogleUserProfile {
    email: String,
}

pub async fn oauth_callback(
    State(AppState {
        db_pool,
        google_oauth,
        http_client,
        secret_store,
        ..
    }): State<crate::AppState>,
    Query(params): Query<OauthCallbackQuery>,
) -> Result<Response, Response> {
    let token = google_oauth
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let profile_resp = http_client
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(token.access_token().secret().to_owned())
        .send()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let profile = profile_resp
        .json::<GoogleUserProfile>()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    sqlx::query!(
        "INSERT INTO accounts (email) VALUES ($1) ON CONFLICT DO NOTHING",
        profile.email
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let account = sqlx::query_as!(
        AccountRecord,
        "SELECT * FROM accounts WHERE email = $1",
        profile.email
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let jwt = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Jwt {
            account,
            exp: OffsetDateTime::now_utc(),
        },
        &jsonwebtoken::EncodingKey::from_secret(
            secret_store.get("SESSION_JWT_SECRET").unwrap().as_bytes(),
        ),
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let mut resp = Redirect::to("/").into_response();
    resp.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(format!("jwt={}", jwt).as_str())
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?,
    );

    Ok(resp)
}
