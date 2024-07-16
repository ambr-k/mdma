use axum::{
    extract::{NestedPath, OriginalUri, Request},
    http::{HeaderMap, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use maud::{html, Markup};

use crate::components;

mod bulk_update;
mod discord_audit;
mod generations;
mod members;

fn home(nest: &str, load_main: Option<Uri>) -> Markup {
    components::layout(
        html! {
            ul ."menu"."menu-horizontal"."navbar-start" {
                li {a hx-get={(nest)"/members"}        hx-target="main" hx-push-url="true" {"Members List"}}
                li {a hx-get={(nest)"/generations"}    hx-target="main" hx-push-url="true" {"Generations"}}
                li {a hx-get={(nest)"/discord_audit"}  hx-target="main" hx-push-url="true" {"Discord Audit"}}
                li {a hx-get={(nest)"/bulk_update"}    hx-target="main" hx-push-url="true" {"Bulk Update"}}
            }
            ul ."menu"."menu-horizontal"."navbar-end" {
                li {a href="/signout" {"Sign Out"}}
            }
        },
        load_main.map(|uri| {
            html! { #"lazy-load-contents" hx-get=(uri) hx-trigger="load" hx-swap="outerHTML" hx-headers=r#"{"X-Rebuild-Page": true}"# { progress ."progress"."mt-6" {} } }
        }),
    )
}

async fn home_no_contents(nest: NestedPath) -> Markup {
    home(nest.as_str(), None)
}

async fn handle_nonhtmx_request(
    headers: HeaderMap,
    nest: NestedPath,
    OriginalUri(original_uri): OriginalUri,
    req: Request,
    next: Next,
) -> Response {
    if headers.contains_key("Hx-Request") {
        next.run(req).await
    } else {
        home(nest.as_str(), Some(original_uri)).into_response()
    }
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/generations", get(generations::generations_list))
        .route("/bulk_update", get(bulk_update::bulk_update_form))
        .route(
            "/.givingfuel_bulk_import",
            post(bulk_update::submit_givingfuel_bulk_update),
        )
        .with_state(state.clone())
        .nest("/members", members::router(state.clone()))
        .nest("/discord_audit", discord_audit::router(state.clone()))
        .layer(middleware::from_fn(handle_nonhtmx_request))
        .route("/", get(home_no_contents))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::verify_admin,
        ))
}
