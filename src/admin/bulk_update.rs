use std::collections::HashSet;

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Response,
    Extension,
};
use rust_decimal::Decimal;

#[derive(Template)]
#[template(path = "admin/bulk_update.html")]
pub struct BulkUpdateTemplate {}

pub async fn bulk_update_form() -> BulkUpdateTemplate {
    BulkUpdateTemplate {}
}

time::serde::format_description!(
    givingfuel_date_format,
    PrimitiveDateTime,
    "[year]-[month]-[day] [hour padding:none repr:12]:[minute] [period]"
);

#[derive(serde::Deserialize, Debug)]
struct GivingFuelDonationRow {
    #[serde(rename = "Transaction ID")]
    transaction_id: i32,
    #[serde(rename = "Total Paid ($ Amount)")]
    total: Option<Decimal>,
    // #[serde(rename = "Currency")]
    #[serde(rename = "Payment Method")]
    payment_method: String,
    // #[serde(rename = "Payment Account")]
    // #[serde(rename = "Expiration Month")]
    // #[serde(rename = "Expiration Year")]
    #[serde(rename = "Payment Date", with = "givingfuel_date_format")]
    payment_date: time::PrimitiveDateTime,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Transaction Type")]
    transaction_type: String,
    // #[serde(rename = "Fund")]
    // #[serde(rename = "Comments")]
    // #[serde(rename = "Tax Deductible ($ Amount)")]
    // #[serde(rename = "Recurring")]
    // #[serde(rename = "Page Name")]
    // #[serde(rename = "Event Selection")]
    // #[serde(rename = "Date Selection")]
    // #[serde(rename = "Timeslot Selection")]
    #[serde(rename = "Billing Name (First Name)")]
    first_name: String,
    #[serde(rename = "Billing Name (Last Name)")]
    last_name: String,
    // #[serde(rename = "Billing Organization Name")]
    // #[serde(rename = "Billing Address (Address 1)")]
    // #[serde(rename = "Billing Address (Address 2)")]
    // #[serde(rename = "Billing Address (City)")]
    // #[serde(rename = "Billing Address (State/Province)")]
    // #[serde(rename = "Billing Address (Country)")]
    // #[serde(rename = "Billing Address (Postal Code)")]
    #[serde(rename = "Billing Email Address")]
    email: String,
    // #[serde(rename = "Billing Email OptIn")]
    // #[serde(rename = "Billing Phone Number")]
    // #[serde(rename = "Reversal Note")]
    // #[serde(rename = "Registration ID")]
    // subscription_id: Option<i32>,
    // #[serde(rename = "Order ID")]
    // #[serde(rename = "Donation ID")]
    // #[serde(rename = "Subscription ID")]
    // #[serde(rename = "Order Number")]
    // #[serde(rename = "Donation Type")]
    // #[serde(rename = "Gateway Label")]
    // #[serde(rename = "Payout Reference ID")]
    // #[serde(rename = "Payout Date")]
    // #[serde(rename = "Payout Total ($ Amount)")]
    // #[serde(rename = "Processing & Fees ($ Amount)")]
    // #[serde(rename = "Gateway")]
    // #[serde(rename = "Gateway Reference")]
    // #[serde(rename = "Admin Notes")]
    // #[serde(rename = "Metadata (key: value)")]
    // #[serde(rename = "Originating Source")]
}

pub async fn submit_givingfuel_bulk_update(
    Extension(user): Extension<crate::auth::Jwt>,
    State(state): State<crate::AppState>,
    mut multipart: Multipart,
) -> Result<Response, Response> {
    let mut email_verified = false;
    let mut csv_text: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name().unwrap() {
            "email-verify" => {
                email_verified = field.text().await.unwrap() == user.account.email;
            }
            "file" => {
                csv_text = Some(field.text().await.unwrap());
            }
            _ => (),
        }
    }

    if !email_verified {
        return Err((StatusCode::BAD_REQUEST, "Email does not match").into_response());
    }
    if csv_text.is_none() {
        return Err((StatusCode::BAD_REQUEST, "Invalid CSV File").into_response());
    }

    let csv_text = csv_text.ok_or((StatusCode::BAD_REQUEST, "Invalid CSV File").into_response())?;
    let mut csv_reader = csv::Reader::from_reader(csv_text.as_bytes());

    let mut emails = HashSet::<String>::from_iter(
        sqlx::query_scalar!(r#"SELECT email FROM members"#)
            .fetch_all(&state.db_pool)
            .await
            .unwrap(),
    );

    let mut transaction = state.db_pool.begin().await.unwrap();

    let mut members_added = 0;
    let mut payments_added = 0;

    for result in csv_reader
        .deserialize::<GivingFuelDonationRow>()
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        let row = result
            .as_ref()
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()).into_response())?;

        if row.status != "completed" || row.transaction_type != "charge" || row.total.is_none() {
            continue;
        }

        if !emails.contains(&row.email) {
            sqlx::query!(
                r#"INSERT INTO members (email, first_name, last_name)
                    VALUES ($1, $2, $3)"#,
                row.email,
                row.first_name,
                row.last_name
            )
            .execute(&mut *transaction)
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

            emails.insert(row.email.clone());
            members_added += 1;
        }

        sqlx::query!(
            r#"INSERT INTO payments (member_id, effective_on, amount_paid, payment_method, platform, transaction_id)
                SELECT id, $2, $3, $4, 'webconnex', $5
                FROM members
                WHERE email = $1"#,
            row.email,
            row.payment_date.date(),
            row.total.unwrap(),
            row.payment_method,
            row.transaction_id
        )
        .execute(&mut *transaction)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

        payments_added += 1;
    }

    transaction
        .commit()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    Ok((
        StatusCode::OK,
        format!(
            "Added {} members and {} payments successfully",
            members_added, payments_added
        ),
    )
        .into_response())
}
