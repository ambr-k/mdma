use axum::{
    extract::{NestedPath, Path, Query, State},
    response::{IntoResponse, Response},
    Form,
};
use lettre::{message::Mailbox, AsyncTransport};
use maud::{html, Markup};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    icons,
    send_email::{build_mailer, build_message, sanitize_email, EmailValues},
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
                (state.persist.load::<String>(&format!("email.{email_key}")).unwrap_or_default())
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
    match state.persist.save(&format!("email.{email_key}"), content) {
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
    html! {
        #"email_addresses_results" {}
        form hx-post={(nest.as_str())"/email_addresses"} hx-target="#email_addresses_results" {
            label ."form-control"."w-full"."max-w-lg"."mx-auto" {
                ."label" { span ."label-text" {"From"} }
                input type="text" name="from_address" value=(state.persist.load::<String>("from_address").unwrap_or_default()) ."input"."input-bordered"."w-full";
            }
            label ."form-control"."w-full"."max-w-lg"."mx-auto" {
                ."label" { span ."label-text" {"Reply-To"} }
                input type="text" name="replyto_address" value=(state.persist.load::<String>("replyto_address").unwrap_or_default()) ."input"."input-bordered"."w-full";
            }
            button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0"."mt-2" {"UPDATE"}
        }
    }
}

#[derive(Deserialize)]
pub struct EmailAddressesFormData {
    from_address: String,
    replyto_address: String,
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
    if let Err(err) = state.persist.save("from_address", form.from_address) {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {(err)}}
        };
    }
    if let Err(err) = state.persist.save("replyto_address", form.replyto_address) {
        return html! {
            ."alert"."alert-error" {(icons::error()) span {(err)}}
        };
    }
    html! {."alert"."alert-success" {(icons::success()) span {"Successfully updated email addresses!"}}}
}

#[derive(Deserialize)]
pub struct SendEmailParams {
    pub first_name: String,
    pub invite_url: String,
    pub email: String,
}

pub async fn send_email(
    Path(email_key): Path<String>,
    State(state): State<crate::AppState>,
    Query(params): Query<SendEmailParams>,
) -> Result<Response, Response> {
    let mailer = build_mailer(&state)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err).into_response())?;
    let values = EmailValues {
        first_name: params.first_name,
        invite_url: params.invite_url,
    };
    let message = build_message(
        &email_key,
        "Psychedelic Club Discord",
        &params.email,
        &values,
        &state,
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err).into_response())?;
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
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}
