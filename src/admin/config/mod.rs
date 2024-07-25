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
            input type="radio" name="config-accordion" hx-get={(nest.as_str())"/email_addresses"} hx-target="next .collapse-content";
            ."collapse-title"."text-xl"."font-medium" {"Email Addresses"}
            ."collapse-content" {}
        }
        ."collapse"."collapse-arrow"."bg-base-200"."my-4"."border"."border-secondary" {
            input type="radio" name="config-accordion" hx-get={(nest.as_str())"/email_contents/discord"} hx-target="next .collapse-content";
            ."collapse-title"."text-xl"."font-medium" {"Discord Email Contents"}
            ."collapse-content" {}
        }
        ."collapse"."collapse-arrow"."bg-base-200"."my-4"."border"."border-secondary" {
            input type="radio" name="config-accordion" hx-get={(nest.as_str())"/email_contents/board_notif"} hx-target="next .collapse-content";
            ."collapse-title"."text-xl"."font-medium" {"Exec Board Notification Email Contents"}
            ."collapse-content" {}
        }
    }}
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route(
            "/email_contents/:email_key",
            get(emails::email_contents_form).post(emails::set_email_contents),
        )
        .route("/send_email/:email_key", get(emails::send_email))
        .route(
            "/email_addresses",
            get(emails::email_addresses_form).post(emails::set_email_addresses),
        )
        .with_state(state.clone())
}
