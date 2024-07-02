use std::{str::FromStr, sync::Arc};

use axum::{
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use shuttle_runtime::{CustomError, SecretStore};
use tower_http::services::ServeDir;

mod admin;
mod auth;
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
}

async fn home(cookies: CookieJar) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                meta http-equiv="X-UA-Compatible" content="ie=edge";
                title {"Membership Database Management Application"}
                link rel="stylesheet" href="./assets/styles.css";
            }
            body {
                header ."navbar"."bg-base-300"."lg:rounded-box"."lg:m-3"."lg:w-auto" {
                    @match cookies.get("jwt") {
                        None => a ."btn" href="signin" {"Sign In"},
                        Some(_) => {
                            ul ."menu"."menu-horizontal"."navbar-start" {
                                li {a hx-get="admin/users"          hx-target="main" {"Users List"}}
                                li {a hx-get="admin/generations"    hx-target="main" {"Generations"}}
                                li {a hx-get="admin/bulk_update"    hx-target="main" {"Bulk Update"}}
                            }
                            ul ."menu"."menu-horizontal"."navbar-end" {
                                li {a href="signout" {"Sign Out"}}
                            }
                        }
                    }
                }
                main ."my-2"."lg:mx-4" {}
                dialog #"modal"."modal"."modal-bottom"."sm:modal-middle" {
                    ."modal-box" {
                        form method="dialog" { button ."btn"."btn-sm"."btn-circle"."btn-ghost"."absolute"."right-2"."top-2" {"✕"} }
                        progress #"modal-loading"."progress"."mt-6"."[&:has(+#modal-content:not(:empty)):not(.htmx-request)]:hidden" {}
                        div #"modal-content" {}
                    }
                    script {(PreEscaped("function openModal() { $('#modal-content').empty(); $('#modal')[0].showModal(); }"))}
                    form method="dialog" ."modal-backdrop" { button {"CLOSE"} }
                }
                #"alerts"."toast" {}
                script src="https://unpkg.com/htmx.org@1.9.9" {}
                script src="https://code.jquery.com/jquery-3.7.1.slim.min.js" {}
            }
        }
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
        secret_store.get("DISCORD_API_KEY").unwrap().as_str(),
    );

    let http_client = reqwest::Client::new();

    let discord_http = Arc::new(serenity::http::Http::new(
        secret_store.get("DISCORD_BOT_TOKEN").unwrap().as_str(),
    ));
    discord_http.set_application_id(
        serenity::model::id::ApplicationId::from_str(
            secret_store.get("DISCORD_APPLICATION_ID").unwrap().as_str(),
        )
        .unwrap(),
    );

    discord::create_commands(&discord_http).await;

    let state = AppState {
        db_pool,
        secret_store,
        google_oauth,
        http_client,
        discord_verifier,
        discord_http,
    };

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
