use askama::Template;
use axum::{extract::State, routing::get, Router};
use shuttle_runtime::CustomError;
use shuttle_secrets::SecretStore;
use tower_http::{services::ServeDir, trace::TraceLayer};
mod admin;
mod auth;
mod webconnex;

#[derive(Clone)]
struct AppState {
    db_pool: sqlx::PgPool,
    secret_store: SecretStore,
    google_oauth: oauth2::basic::BasicClient,
    http_client: reqwest::Client,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    title: &'a str,
    oauth_id: String,
    oauth_redirect: String,
}

async fn home(State(state): State<AppState>) -> HomeTemplate<'static> {
    HomeTemplate {
        title: "Home",
        oauth_id: state.secret_store.get("GOOGLE_OAUTH_CLIENT_ID").unwrap(),
        oauth_redirect: state.secret_store.get("GOOGLE_OAUTH_REDIRECT").unwrap(),
    }
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] db_pool: sqlx::PgPool,
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
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

    let state = AppState {
        db_pool,
        secret_store,
        google_oauth,
        http_client: reqwest::Client::new(),
    };

    let router = Router::new()
        .route("/", get(home))
        .route("/signin", get(auth::signin_redirect))
        .route("/callback-google", get(auth::oauth_callback))
        .with_state(state.clone())
        .nest("/admin", admin::router(state.clone()))
        .nest("/.webconnex", webconnex::router(state.clone()))
        .nest_service("/assets", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http());

    Ok(router.into())
}
