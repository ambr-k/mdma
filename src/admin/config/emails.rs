use axum::{
    extract::{NestedPath, Query, State},
    response::{IntoResponse, Response},
    Form,
};
use maud::{html, Markup, PreEscaped};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{icons, send_email::sanitize_email};

pub fn discord_email_form(nest: NestedPath) -> Markup {
    html! {
        ."collapse-title"."text-xl"."font-medium" {"Discord Invite Email"}
        ."collapse-content" {
            #"discord_email_results" {}
            form hx-post={(nest.as_str())"/discord_email"} hx-target="#discord_email_results" {
                textarea name="email_body" ."textarea"."textarea-primary"."font-mono"."my-4"."w-full" required
                    hx-get={(nest.as_str())"/discord_email"} hx-swap="textContent" hx-trigger="load" hx-target="this" {}
                button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0" {"UPDATE"}
            }
        }
    }
}

pub async fn get_discord_email(State(state): State<crate::AppState>) -> String {
    html! {(state.persist.load::<String>("discord_email").unwrap_or_default())}.into()
}

#[derive(Deserialize)]
pub struct DiscordEmailFormData {
    email_body: String,
}

pub async fn set_discord_email(
    State(state): State<crate::AppState>,
    Form(form): Form<DiscordEmailFormData>,
) -> Markup {
    let content = sanitize_email(&form.email_body);
    match state.persist.save("discord_email", content) {
        Ok(_) => html! {
            ."alert"."alert-success" {(icons::success()) span {"Successfully updated email contents!"}}
        },
        Err(err) => html! {
            ."alert"."alert-error" {(icons::error()) span {(err)}}
        },
    }
}

pub async fn populate_discord_email(
    State(state): State<crate::AppState>,
    Query(params): Query<crate::send_email::EmailValues>,
) -> Response {
    match crate::send_email::populate_discord_email(&params, &state.persist) {
        Ok(populated) => PreEscaped(populated).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}
