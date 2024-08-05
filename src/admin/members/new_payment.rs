use axum::{
    extract::{NestedPath, Path, State},
    response::{IntoResponse, Response},
    Form,
};
use maud::{html, Markup, PreEscaped};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;

use crate::{db::members::MemberRow, icons};

pub async fn payment_form(
    nest: NestedPath,
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let member = sqlx::query_as!(
        MemberRow,
        "SELECT members.* FROM members WHERE members.id=$1",
        member_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    Ok(html! {
        h1 ."font-bold"."text-xl" {"Add Payment: "(member.last_name)", "(member.first_name)}
        h2 ."text-lg" {(member.email)}
        ."form-response" {}
        ."divider" {}
        form ."mt-3" hx-post={(nest.as_str())"/new_payment/"(member.id)} hx-target="previous .form-response" hx-indicator="#modal-loading" {
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Payment Method / Reason"}
                    select name="payment_method" required ."select"."select-primary" {
                        option value="cash" {"Cash"}
                        option value="card" {"Card"}
                        option value="volunteer" {"Volunteering"}
                        option value="grace-period" {"Grace Period"}
                        option value="ethics-committee" {"Ethics Committee"}
                        option value="exec-board" {"Executive Board"}
                        option value="other" {"Other"}
                    }
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Amount Paid"}
                    input type="number" name="amount_paid" required min="0" step="any" value="0.00" ."input"."input-bordered";
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Effective On"}
                    input type="date" name="effective_on" required #"modal_add_payment__effective_on" ."input"."input-bordered";
                    script {"$('#modal_add_payment__effective_on')[0].valueAsDate = new Date();"}
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Duration (Months)"}
                    input type="number" name="duration_months" required min="1" step="1" value="1" ."input"."input-bordered";
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Notes"}
                    textarea name="notes" placeholder="Notes" ."textarea"."textarea-bordered" {}
                }
            }
            ."form-control" { button ."btn"."btn-outline"."btn-primary"."w-1/2"."mx-auto" {"SUBMIT"} }
        }
    })
}

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
            div hx-swap-oob="afterbegin:#alerts" {
                #{"alert_payment_success_"(transaction_id)}."alert"."alert-success"."transition-opacity"."duration-300" role="alert" {
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
