use crate::{db::members::MemberRow, icons};
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::Response,
};
use maud::{html, Markup, PreEscaped};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use sqlx::Error;
use time::Date;

#[derive(Template)]
#[template(path = "admin/user_add_payment.html")]
pub struct NewPaymentFormTemplate {
    user: MemberRow,
}

pub async fn user_payment_form(
    Path(user_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<NewPaymentFormTemplate, Response> {
    let user = sqlx::query_as!(
        MemberRow,
        r#" SELECT members.*, generations.title AS generation_name
            FROM members
                INNER JOIN member_generations ON members.id = member_id
                INNER JOIN generations ON generations.id = generation_id
            WHERE members.id=$1"#,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    Ok(NewPaymentFormTemplate { user })
}

#[serde_inline_default]
#[derive(Deserialize)]
pub struct NewPaymentFormData {
    payment_method: String,
    amount_paid: Option<Decimal>,
    transaction_id: Option<i32>,
    effective_on: Date,
    duration_months: i32,
    notes: Option<String>,
}

pub async fn add_payment(
    State(state): State<crate::AppState>,
    Path(user_id): Path<i32>,
    Form(form): Form<NewPaymentFormData>,
) -> Markup {
    let sql_result = sqlx::query_scalar!(
        r#"INSERT INTO payments (member_id, effective_on, duration_months, amount_paid, payment_method, platform, transaction_id, notes)
            VALUES              ($1,        $2,           $3,              $4,          $5,             'mdma',   $6,             $7)
            RETURNING id"#,
        user_id,
        form.effective_on,
        form.duration_months,
        form.amount_paid.unwrap_or(Decimal::ZERO),
        form.payment_method,
        form.transaction_id,
        form.notes
    ).fetch_one(&state.db_pool)
    .await;

    match sql_result {
        Ok(transaction_id) => html! {
            div hx-swap-oob={"innerHTML:#user_details_"(user_id)} {
                progress ."progress"."htmx-indicator" {
                    script {(PreEscaped(format!("
                        $('#modal')[0].close();
                        htmx.trigger('#user_details_trigger_{}', 'change', {{}});
                    ", user_id)))}
                }
            }
            div hx-swap-oob="beforeend:#alerts" {
                #{"alert_payment_success_"(transaction_id)}."alert"."alert-success"."transition-opacity"."duraiton-300" role="alert" {
                    (icons::success())
                    span {"Payment Added Successfully"}
                    script {(PreEscaped(format!("
                        setTimeout(() => {{
                            const toastElem = $('#alert_payment_success_{}');
                            toastElem.on('transitionend', (event) => {{event.target.remove();}});
                            toastElem.css('opacity', 0);
                        }}, 2500);
                    ", transaction_id)))}
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
