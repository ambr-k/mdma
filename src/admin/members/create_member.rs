use std::str::FromStr;

use axum::{
    extract::{NestedPath, State},
    response::{IntoResponse, Response},
    Form,
};
use maud::{html, Markup};
use serde::Deserialize;

use crate::{components, err_responses::MapErrorResponse, icons};

pub async fn member_form(nest: NestedPath) -> Markup {
    html! {
        h1 ."font-bold"."text-xl" {"Create New Member"}
        ."form-response" {}
        ."divider" {}
        form hx-post={(nest.as_str())"/create"} hx-target="previous .form-response" hx-indicator="#modal-loading" {
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"First Name"}
                }
                input type="text" name="first_name" required ."input"."input-bordered"."w-full";
            }
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"Last Name"}
                }
                input type="text" name="last_name" required ."input"."input-bordered"."w-full";
            }
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"Email"}
                }
                input type="email" name="email" required ."input"."input-bordered"."w-full";
            }
            ."form-control"."mt-4" { button ."btn"."btn-outline"."btn-primary"."w-1/2"."mx-auto" {"SUBMIT"} }
        }
    }
}

#[derive(Deserialize)]
pub struct CreateMemberFormData {
    first_name: String,
    last_name: String,
    email: String,
}

pub async fn add_member(
    nest: NestedPath,
    State(state): State<crate::AppState>,
    Form(form): Form<CreateMemberFormData>,
) -> Result<Markup, Response> {
    lettre::Address::from_str(&form.email).map_err_response(
        crate::err_responses::ErrorResponse::AlertWithPrelude("Invalid Email"),
    )?;

    sqlx::query!(
        r#"INSERT INTO members  (first_name,    last_name,  email)
            VALUES              ($1,            $2,         $3)"#,
        form.first_name,
        form.last_name,
        form.email.to_lowercase()
    )
    .execute(&state.db_pool)
    .await
    .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        #"reload-list" hx-get={(nest.as_str())} hx-vals=(format!(r#"{{"search": "{}"}}"#, form.email)) hx-target="#members-list" hx-trigger="load" hx-swap="outerHTML" { progress ."progress"."mt-6" {} }
        script { "$('#modal')[0].close();" }

        (components::ToastAlert::Success(&format!("{}, {} Added Successfully", form.last_name, form.first_name)))
    })
}
