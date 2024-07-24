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
            input type="radio" name="config-accordion" hx-get={(nest.as_str())"/discord_email"} hx-target="next .collapse-content";
            ."collapse-title"."text-xl"."font-medium" {"Discord Email Contents"}
            ."collapse-content" {}
        }
    }}
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route(
            "/discord_email",
            get(emails::discord_email_form).post(emails::set_discord_email),
        )
        .route("/discord_email/send", get(emails::send_discord_email))
        .route(
            "/email_addresses",
            get(emails::email_addresses_form).post(emails::set_email_addresses),
        )
        .with_state(state.clone())
}
