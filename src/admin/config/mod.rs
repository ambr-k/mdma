use axum::{extract::NestedPath, routing::get, Router};
use maud::{html, Markup};

use crate::icons;

mod emails;

async fn home(nest: NestedPath) -> Markup {
    html! { #"mdma-config" ."w-full"."max-w-4xl"."mx-auto" {
        ."alert"."alert-warning"."w-full"."max-w-xl"."mx-auto" role="warning" {
            (icons::warning())
            span {"Warning: Here be dragons! 🐉 Seriously, make sure you know what you're doing on this page..."}
        }
        ."collapse"."collapse-arrow"."bg-base-200"."my-4"."border"."border-secondary" {
            input type="radio" name="config-accordion";
            (emails::discord_email_form(nest))
        }
    }}
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route(
            "/discord_email",
            get(emails::get_discord_email).post(emails::set_discord_email),
        )
        .with_state(state.clone())
}
