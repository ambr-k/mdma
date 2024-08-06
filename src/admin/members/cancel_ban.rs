use axum::{
    extract::{NestedPath, Path, State},
    response::Response,
    Extension, Form,
};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use crate::{components, db::members::MemberRow, err_responses::MapErrorResponse, icons};

#[derive(Deserialize)]
pub struct CancelFormData {
    #[serde(default)]
    reason: String,
}

pub async fn cancel_form(
    nest: NestedPath,
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let member = sqlx::query_as!(MemberRow, "SELECT * FROM members WHERE id = $1", member_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        h1 ."font-bold"."text-xl" {"Cancel Member: "(member.last_name)", "(member.first_name)}
        ."form-response" {}
        ."divider" {}
        form ."mt-3" hx-post={(nest.as_str())"/cancel/"(member.id)} hx-target="previous .form-response" hx-indicator="#modal-loading" {
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"Cancellation Reason"}
                }
                input type="text" name="reason" ."input"."input-bordered"."w-full";
                ."alert"."alert-warning"."mt-4"."w-full" role="warning" {
                    (icons::warning())
                    span {"Cancelling through MDMA does not cancel any subscriptions on third-party payment services."}
                }
                ."form-control"."mt-4" { button ."btn"."btn-outline"."btn-primary"."w-1/2"."mx-auto" {(icons::warning())" SUBMIT"} }
            }
        }
    })
}

pub async fn cancel_member(
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
    Extension(admin): Extension<crate::auth::Jwt>,
    Form(CancelFormData { reason }): Form<CancelFormData>,
) -> Result<Markup, Response> {
    sqlx::query!(
        r#" UPDATE members
            SET
                cancelled = TRUE,
                notes = TRIM(E'\n' FROM notes || E'\n\n=== ' || CURRENT_DATE || E' ===\n' || $2)
            WHERE id = $1"#,
        member_id,
        format!(
            "Cancelled by admin {} : {}",
            admin.account.email,
            if reason.trim().is_empty() {
                "(No reason given)"
            } else {
                reason.trim()
            }
        )
    )
    .execute(&state.db_pool)
    .await
    .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        div hx-swap-oob={"innerHTML:#user_details_"(member_id)} {
            progress ."progress"."htmx-indicator" {
                script {(PreEscaped(format!("
                        $('#modal')[0].close();
                        htmx.trigger('#user_details_trigger_{}', 'change', {{}});
                    ", member_id)))}
            }
        }
        (components::ToastAlert::Success("Cancelled Successfully"))
    })
}

pub async fn ban_form(
    nest: NestedPath,
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let member = sqlx::query_as!(MemberRow, "SELECT * FROM members WHERE id = $1", member_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        h1 ."font-bold"."text-xl" {"Ban Member: "(member.last_name)", "(member.first_name)}
        ."form-response" {}
        ."divider" {}
        form ."mt-3" hx-post={(nest.as_str())"/ban/"(member.id)} hx-target="previous .form-response" hx-indicator="#modal-loading" {
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"Ban Reason"}
                }
                input type="text" required name="reason" ."input"."input-bordered"."w-full";
                ."alert"."alert-warning"."mt-4"."w-full" role="warning" {
                    (icons::warning())
                    span {"Banning through MDMA does not cancel any subscriptions on third-party payment services."}
                }
                ."form-control"."mt-4" { button ."btn"."btn-outline"."btn-primary"."w-1/2"."mx-auto" {(icons::warning())" SUBMIT"} }
            }
        }
    })
}

pub async fn ban_member(
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
    Extension(admin): Extension<crate::auth::Jwt>,
    Form(CancelFormData { reason }): Form<CancelFormData>,
) -> Result<Markup, Response> {
    if reason.trim().is_empty() {
        return Err("Must provide a reason")
            .map_err_response(crate::err_responses::ErrorResponse::Alert);
    }

    sqlx::query!(
        r#" UPDATE members
            SET
                banned = TRUE,
                notes = TRIM(E'\n' FROM notes || E'\n\n=== ' || CURRENT_DATE || E' ===\n' || $2)
            WHERE id = $1"#,
        member_id,
        format!(
            "Banned by admin {} : {}",
            admin.account.email,
            reason.trim(),
        )
    )
    .execute(&state.db_pool)
    .await
    .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        div hx-swap-oob={"innerHTML:#user_details_"(member_id)} {
            progress ."progress"."htmx-indicator" {
                script {(PreEscaped(format!("
                        $('#modal')[0].close();
                        htmx.trigger('#user_details_trigger_{}', 'change', {{}});
                    ", member_id)))}
            }
        }
        (components::ToastAlert::Success("Banned Member Successfully"))
    })
}

pub async fn unban_form(
    nest: NestedPath,
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let member = sqlx::query_as!(MemberRow, "SELECT * FROM members WHERE id = $1", member_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        h1 ."font-bold"."text-xl" {"Unban Member: "(member.last_name)", "(member.first_name)}
        ."form-response" {}
        ."divider" {}
        form ."mt-3" hx-post={(nest.as_str())"/unban/"(member.id)} hx-target="previous .form-response" hx-indicator="#modal-loading" {
            ."form-control"."w-full" {
                ."label" {
                    span ."label-text" {"Unban Reason"}
                }
                input type="text" required name="reason" ."input"."input-bordered"."w-full";
                ."form-control"."mt-4" { button ."btn"."btn-outline"."btn-primary"."w-1/2"."mx-auto" {(icons::warning())" SUBMIT"} }
            }
        }
    })
}

pub async fn unban_member(
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
    Extension(admin): Extension<crate::auth::Jwt>,
    Form(CancelFormData { reason }): Form<CancelFormData>,
) -> Result<Markup, Response> {
    if reason.trim().is_empty() {
        return Err("Must provide a reason")
            .map_err_response(crate::err_responses::ErrorResponse::Alert);
    }

    sqlx::query!(
        r#" UPDATE members
            SET
                banned = FALSE,
                notes = TRIM(E'\n' FROM notes || E'\n\n=== ' || CURRENT_DATE || E' ===\n' || $2)
            WHERE id = $1"#,
        member_id,
        format!(
            "Unbanned by admin {} : {}",
            admin.account.email,
            reason.trim(),
        )
    )
    .execute(&state.db_pool)
    .await
    .map_err_response(crate::err_responses::ErrorResponse::Alert)?;

    Ok(html! {
        div hx-swap-oob={"innerHTML:#user_details_"(member_id)} {
            progress ."progress"."htmx-indicator" {
                script {(PreEscaped(format!("
                        $('#modal')[0].close();
                        htmx.trigger('#user_details_trigger_{}', 'change', {{}});
                    ", member_id)))}
            }
        }
        (components::ToastAlert::Success("Unbanned Member Successfully"))
    })
}
