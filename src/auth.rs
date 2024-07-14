use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use jsonwebtoken::Validation;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use time::Duration;

use crate::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct AccountRecord {
    pub id: i32,
    pub email: String,
    pub is_admin: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Jwt {
    pub account: AccountRecord,
    pub exp: u64,
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
    cookies: CookieJar,
    Query(params): Query<OauthCallbackQuery>,
) -> Result<(CookieJar, Redirect), Response> {
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
    .execute(&db_pool)
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
            exp: jsonwebtoken::get_current_timestamp()
                + Duration::days(7).whole_seconds().unsigned_abs(),
        },
        &jsonwebtoken::EncodingKey::from_secret(
            secret_store.get("SESSION_JWT_SECRET").unwrap().as_bytes(),
        ),
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    Ok((cookies.add(Cookie::new("jwt", jwt)), Redirect::to("/")))
}

pub async fn verify_admin(
    State(AppState { secret_store, .. }): State<crate::AppState>,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let jwt = cookies
        .get("jwt")
        .ok_or(StatusCode::UNAUTHORIZED.into_response())?
        .value();

    let claims = jsonwebtoken::decode::<Jwt>(
        jwt,
        &jsonwebtoken::DecodingKey::from_secret(
            secret_store.get("SESSION_JWT_SECRET").unwrap().as_bytes(),
        ),
        &Validation::default(),
    )
    .map_err(|err| {
        (
            cookies.to_owned().remove("jwt"),
            (StatusCode::UNAUTHORIZED, err.to_string()),
        )
            .into_response()
    })?
    .claims;

    if !claims.account.is_admin {
        return Err((StatusCode::FORBIDDEN, "Not an admin").into_response());
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn signout(cookies: CookieJar) -> (CookieJar, Redirect) {
    return (cookies.remove("jwt"), Redirect::to("/"));
}
