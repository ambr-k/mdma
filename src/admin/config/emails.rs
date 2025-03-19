use axum::{
    extract::{NestedPath, Path, Query, State},
    response::{IntoResponse, Response},
    Form,
};
use lettre::{message::Mailbox, AsyncTransport};
use maud::{html, Markup};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::try_join;

use crate::{
    err_responses::{ErrorResponse, MapErrorResponse},
    icons,
    send_email::{
        build_mailer, build_message, get_email_address, get_email_template, insert_email_address,
        insert_email_template, sanitize_email, EmailValues,
    },
};

pub async fn email_contents_form(
    nest: NestedPath,
    Path(email_key): Path<String>,
    State(state): State<crate::AppState>,
) -> Markup {
    html! {
        #{(email_key)"_email_results"} {}
        form hx-post={(nest.as_str())"/email_contents/"(email_key)} hx-target={"#"(email_key)"_email_results"} {
            textarea name="email_body" ."textarea"."textarea-primary"."font-mono"."my-4"."w-full" required {
                (get_email_template(&email_key, &state.db_pool).await.unwrap_or_default())
            }
            button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0" {"UPDATE"}
        }
    }
}

#[derive(Deserialize)]
pub struct EmailFormData {
    email_body: String,
}

pub async fn set_email_contents(
    Path(email_key): Path<String>,
    State(state): State<crate::AppState>,
    Form(form): Form<EmailFormData>,
) -> Markup {
    let content = sanitize_email(&form.email_body);
    match insert_email_template(&email_key, &content, &state.db_pool).await {
        Ok(_) => html! {
            ."alert"."alert-success" {(icons::success()) span {"Successfully updated email contents!"}}
        },
        Err(err) => html! {
            ."alert"."alert-error" {(icons::error()) span {(err)}}
        },
    }
}

pub async fn email_addresses_form(
    nest: NestedPath,
    State(state): State<crate::AppState>,
) -> Markup {
    let (from_addr, replyto_addr, board_notif_addr) = try_join!(
        get_email_address("from", &state.db_pool),
        get_email_address("replyto", &state.db_pool),
        get_email_address("board_notif", &state.db_pool)
    )
    .unwrap_or_default();
    html! {
        #"email_addresses_results" {}
        form hx-post={(nest.as_str())"/email_addresses"} hx-target="#email_addresses_results" {
            label ."form-control"."w-full"."max-w-lg"."mx-auto" {
                ."label" { span ."label-text" {"From"} }
                input type="text" name="from_address" value=(from_addr) ."input"."input-bordered"."w-full";
            }
            label ."form-control"."w-full"."max-w-lg"."mx-auto" {
                ."label" { span ."label-text" {"Reply-To"} }
                input type="text" name="replyto_address" value=(replyto_addr) ."input"."input-bordered"."w-full";
            }
            label ."form-control"."w-full"."max-w-lg"."mx-auto" {
                ."label" { span ."label-text" {"Board Notification"} }
                input type="text" name="board_notif_address" value=(board_notif_addr) ."input"."input-bordered"."w-full";
            }
            button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0"."mt-2" {"UPDATE"}
        }
    }
}

#[derive(Deserialize)]
pub struct EmailAddressesFormData {
    from_address: String,
    replyto_address: String,
    board_notif_address: String,
}

pub async fn set_email_addresses(
    State(state): State<crate::AppState>,
    Form(form): Form<EmailAddressesFormData>,
) -> Markup {
    if let Err(err) = form.from_address.parse::<Mailbox>() {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {"Invalid 'From' Address: "(err)}}
        };
    }
    if let Err(err) = form.replyto_address.parse::<Mailbox>() {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {"Invalid 'Reply-To' Address: "(err)}}
        };
    }
    if let Err(err) = form.board_notif_address.parse::<Mailbox>() {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {"Invalid 'Board Notification' Address: "(err)}}
        };
    }
    if let Err(err) = try_join!(
        insert_email_address("from", &form.from_address, &state.db_pool),
        insert_email_address("replyto", &form.replyto_address, &state.db_pool),
        insert_email_address("board_notif", &form.board_notif_address, &state.db_pool),
    ) {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {(err)}}
        };
    }
    html! {."alert"."alert-success" {(icons::success()) span {"Successfully updated email addresses!"}}}
}

pub async fn send_email(
    Path(email_key): Path<String>,
    State(state): State<crate::AppState>,
    Query(params): Query<EmailValues>,
) -> Result<Response, Response> {
    let mailer = build_mailer(&state)
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;
    let message = build_message(
        &email_key,
        "Psychedelic Club Discord",
        &params.email,
        &params,
        &state,
    )
    .await
    .map_err_response(ErrorResponse::InternalServerError)?;
    mailer
        .send(message)
        .await
        .map(|resp| {
            (
                StatusCode::OK,
                format!(
                    "{} {}",
                    resp.code().to_string(),
                    resp.first_line().unwrap_or_default()
                ),
            )
                .into_response()
        })
        .map_err_response(ErrorResponse::InternalServerError)
}
