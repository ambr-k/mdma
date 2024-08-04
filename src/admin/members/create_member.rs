use std::str::FromStr;

use axum::{
    extract::{NestedPath, State},
    Form,
};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use crate::icons;

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
) -> Markup {
    if let Err(err) = lettre::Address::from_str(&form.email) {
        return html! { ."alert"."alert-error" role="alert" {
            (icons::error())
            span {"Invalid Email: "(err.to_string())}
        } };
    }

    match sqlx::query_scalar!(
        r#"INSERT INTO members  (first_name,    last_name,  email)
            VALUES              ($1,            $2,         $3)
            RETURNING id"#,
        form.first_name,
        form.last_name,
        form.email.to_lowercase()
    )
    .fetch_one(&state.db_pool)
    .await
    {
        Ok(user_id) => html! {
            {
                #"reload-list" hx-get={(nest.as_str())} hx-vals=(format!(r#"{{"search": "{}"}}"#, form.email)) hx-target="#members-list" hx-trigger="load" hx-swap="outerHTML" { progress ."progress"."mt-6" {} }
                script { "$('#modal')[0].close();" }
            }

            div hx-swap-oob="afterbegin:#alerts" {
                #{"alert_newmember_success_"(user_id)}."alert"."alert-success"."transition-opacity"."duration-300" role="alert" {
                    (icons::success())
                    span {"Member Added Successfully"}
                    script {(PreEscaped(format!("
                        setTimeout(() => {{
                            const toastElem = $('#alert_newmember_success_{}');
                            toastElem.on('transitionend', (event) => {{event.target.remove();}});
                            toastElem.css('opacity', 0);
                        }}, 2500);
                    ", user_id)))}
                }
            }
        },

        Err(err) => html! {
            ."alert"."alert-error" role="alert" {
                (icons::error())
                span {(err.to_string())}
            }
        },
    }
}
