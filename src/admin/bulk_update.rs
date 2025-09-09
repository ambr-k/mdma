use std::collections::HashSet;

use crate::{
    err_responses::{ErrorResponse, MapErrorResponse},
    icons,
};
use axum::{
    extract::{Multipart, NestedPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
};
use maud::{html, Markup};
use rust_decimal::Decimal;

pub async fn bulk_update_form(nest: NestedPath) -> Markup {
    html! {
        ."alert"."alert-warning"."w-full"."max-w-xl"."mx-auto" role="warning" {
            (icons::warning())
            span {"Warning: Here be dragons! 🐉 Seriously, make sure you know what you're doing on this page..."}
        }
        ."collapse"."collapse-arrow"."bg-base-200"."w-full"."max-w-xl"."mx-auto"."mt-4"."outline"."outline-1" {
            input type="checkbox";
            ."collapse-title"."text-xl"."font-medium" {"GivingFuel Donations Import"}
            ."collapse-content" {
                a href="https://manage.webconnex.com/reports/donations" target="_blank" ."btn"."btn-neutral" {"Open Donations Page"}
                p {"Click \"Export\" in the top right of the donations page to download all donation records"}
                form #"givingfuel-bulk-import-form"."mt-8" hx-encoding="multipart/form-data" hx-post={(nest.as_str())"/.givingfuel_bulk_import"} {
                    input type="file" name="file" ."file-input"."file-input-bordered"."file-input-primary"."w-full";
                    label ."form-control"."w-full" {
                        ."label" { span ."label-text" {"Enter your email to prove you know what you're doing..."} }
                        input type="text" name="email-verify" placeholder="Email" ."input"."input-bordered";
                        button ."btn"."btn-secondary"."w-1/3"."mx-auto"."mt-4" {"UPLOAD"}
                    }
                }
            }
        }
        ."collapse"."collapse-arrow"."bg-base-200"."w-full"."max-w-xl"."mx-auto"."mt-4"."outline"."outline-1" {
            input type="checkbox";
            ."collapse-title"."text-xl"."font-medium" {"Donorbox Donations Import"}
            ."collapse-content" {
                form #"donorbox-bulk-import-form" hx-encoding="multipart/form-data" hx-post={(nest.as_str())"/.donorbox_bulk_import"} {
                    label ."form-control"."w-full" {
                        ."label"."cursor-pointer" { span ."label-text" {"Start Date"} }
                        input type="date" name="start-date" required #"donorbox_bulk_import__start_date" ."input"."input-bordered"."cursor-pointer";
                        script {"$('#donorbox_bulk_import__start_date')[0].valueAsDate = new Date();"}
                    }
                    label ."form-control"."w-full" {
                        ."label" { span ."label-text" {"Enter your email to prove you know what you're doing..."} }
                        input type="text" name="email-verify" placeholder="Email" ."input"."input-bordered";
                        button ."btn"."btn-secondary"."w-1/3"."mx-auto"."mt-4" {"IMPORT"}
                    }
                }
            }
        }
    }
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
    // #[serde(rename = "Payment Method")]
    // payment_method: String,
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
            .map_err_response(ErrorResponse::StatusCode(StatusCode::BAD_REQUEST))?;

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
            .map_err_response(ErrorResponse::InternalServerError)?;

            emails.insert(row.email.clone());
            members_added += 1;
        }

        sqlx::query!(
            r#"INSERT INTO payments (member_id, effective_on, amount_paid, payment_method, transaction_id)
                SELECT               id,        $2,           $3,          'webconnex',    $4
                FROM members
                WHERE email = $1"#,
            row.email,
            row.payment_date.date(),
            row.total.unwrap(),
            row.transaction_id
        )
        .execute(&mut *transaction)
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;

        payments_added += 1;
    }

    transaction
        .commit()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;

    Ok((
        StatusCode::OK,
        format!(
            "Added {} members and {} payments successfully",
            members_added, payments_added
        ),
    )
        .into_response())
}

pub async fn submit_donorbox_bulk_update(
    Extension(user): Extension<crate::auth::Jwt>,
    State(state): State<crate::AppState>,
    mut multipart: Multipart,
) -> Result<Response, Response> {
    let mut email_verified = false;
    let mut start_date: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name().unwrap() {
            "email-verify" => {
                email_verified = field.text().await.unwrap() == user.account.email;
            }
            "start-date" => {
                start_date = Some(field.text().await.unwrap());
            }
            _ => (),
        }
    }

    if !email_verified {
        return Err((StatusCode::BAD_REQUEST, "Email does not match").into_response());
    }
    if start_date.is_none() {
        return Err((StatusCode::BAD_REQUEST, "Invalid Start Date").into_response());
    }

    let mut new_members: u32 = 0;
    let mut transactions: u32 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut page = 1;
    loop {
        let donations = state
            .http_client
            .get("https://donorbox.org/api/v1/donations")
            .basic_auth(
                state.secret_store.get("DONORBOX_APILOGIN").unwrap(),
                state.secret_store.get("DONORBOX_APIKEY"),
            )
            .query(&[
                ("date_from", start_date.clone().unwrap()),
                ("page", page.to_string()),
            ])
            .send()
            .await
            .map_err_response(ErrorResponse::InternalServerError)?
            .json::<Vec<crate::donorbox::new_donation::DonationEvent>>()
            .await
            .map_err_response(ErrorResponse::InternalServerError)?;

        if donations.is_empty() {
            break;
        }

        for don in donations {
            match crate::donorbox::new_donation::process_donation(&state, &don, false).await {
                Ok(crate::donorbox::new_donation::ResponseBody {
                    created_member_id: created,
                    ..
                }) => {
                    transactions += 1;
                    if created.is_some() {
                        new_members += 1;
                    }
                }
                Err(err) => {
                    errors.push(format!(
                        "{}: {}",
                        don.id,
                        String::from_utf8(
                            axum::body::to_bytes(err.into_body(), usize::MAX)
                                .await
                                .unwrap()
                                .to_vec(),
                        )
                        .unwrap(),
                    ));
                }
            };
        }

        page += 1;
    }

    let resp = if errors.is_empty() {
        format!(
            "Added {} members and {} payments successfully",
            new_members, transactions
        )
    } else {
        let mut wip = format!(
            "Added {} members and {} payments with {} errors",
            new_members,
            transactions,
            errors.len()
        );
        for err in &errors {
            wip += &("<br>".to_string() + err);
        }
        wip
    };

    Ok((StatusCode::OK, resp).into_response())
}
