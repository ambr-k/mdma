use std::{str::FromStr, sync::Arc};

use axum::{
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use maud::html;
use shuttle_runtime::{CustomError, SecretStore};
use tower_http::services::ServeDir;

mod admin;
mod auth;
mod components;
mod db;
mod discord;
mod icons;
mod webconnex;

#[derive(Clone)]
struct AppState {
    db_pool: sqlx::PgPool,
    secret_store: SecretStore,
    google_oauth: oauth2::basic::BasicClient,
    http_client: reqwest::Client,
    discord_verifier: serenity::interactions_endpoint::Verifier,
    discord_http: Arc<serenity::http::Http>,
    discord_guild: serenity::model::id::GuildId,
}

async fn home(cookies: CookieJar) -> Response {
    match cookies.get("jwt") {
        None => components::layout(
            html! {
                a ."btn" href="/signin" {"Sign In"}
            },
            None,
        )
        .into_response(),
        Some(_) => Redirect::to("/admin").into_response(),
    }
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] db_pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!()
        .run(&db_pool)
        .await
        .map_err(CustomError::new)?;

    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::DEBUG)
    //     .init();

    let google_oauth = auth::oauth_client(
        secret_store.get("GOOGLE_OAUTH_CLIENT_ID").unwrap(),
        secret_store.get("GOOGLE_OAUTH_CLIENT_SECRET").unwrap(),
        secret_store.get("GOOGLE_OAUTH_REDIRECT").unwrap(),
    );

    let discord_verifier = serenity::interactions_endpoint::Verifier::new(
        &secret_store.get("DISCORD_API_KEY").unwrap(),
    );

    let http_client = reqwest::Client::new();

    let discord_http = Arc::new(serenity::http::Http::new(
        &secret_store.get("DISCORD_BOT_TOKEN").unwrap(),
    ));
    discord_http.set_application_id(
        serenity::model::id::ApplicationId::from_str(
            &secret_store.get("DISCORD_APPLICATION_ID").unwrap(),
        )
        .unwrap(),
    );

    let discord_guild =
        serenity::model::id::GuildId::from_str(&secret_store.get("DISCORD_GUILD_ID").unwrap())
            .unwrap();

    let state = AppState {
        db_pool,
        secret_store,
        google_oauth,
        http_client,
        discord_verifier,
        discord_http,
        discord_guild,
    };

    discord::create_commands(&state).await;

    let router = Router::new()
        .route("/", get(home))
        .route("/signin", get(auth::signin_redirect))
        .route("/signout", get(auth::signout))
        .route("/callback-google", get(auth::oauth_callback))
        .route("/.discord/interaction", post(discord::handle_request))
        .with_state(state.clone())
        .nest("/admin", admin::router(state.clone()))
        .nest("/.webconnex", webconnex::router(state.clone()))
        .nest_service("/assets", ServeDir::new("static"));

    Ok(router.into())
}
