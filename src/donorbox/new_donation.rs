use axum::{extract::State, response::Response, Json};
use lettre::AsyncTransport;
use reqwest::StatusCode;
use rust_decimal::Decimal;
use time::OffsetDateTime;
use tokio::try_join;

use crate::{
    discord::create_invite,
    err_responses::{ErrorResponse, MapErrorResponse},
    send_email::{build_mailer, build_message, EmailValues},
};

#[derive(serde::Deserialize)]
struct Campaign {
    id: i32,
}

#[derive(serde::Deserialize, Clone, Default)]
#[allow(dead_code)]
struct Question {
    question: String,
    answer: String,
}

#[derive(serde::Deserialize)]
pub struct DonationEvent {
    action: String,
    campaign: Campaign,
    donor: super::Donor,
    net_amount: Decimal,
    id: i32,
    formatted_net_amount: String,
    stripe_charge_id: String,
    #[serde(with = "time::serde::iso8601")]
    donation_date: OffsetDateTime,
    plan_id: i32,
    questions: Vec<Question>,
}

#[derive(serde::Serialize)]
struct InsertTransactionResult {
    id: i32,
    member_id: i32,
}

#[derive(serde::Serialize)]
pub struct ResponseBody {
    created_member_id: Option<i32>,
    inserted_transaction: InsertTransactionResult,
}

async fn send_emails(state: &crate::AppState, event: &DonationEvent) -> Result<(), Response> {
    let invite_url = create_invite(
        Some(&format!(
            "New member automated invite (Donorbox transaction #{}, Email {})",
            event.id, event.donor.email
        )),
        state,
    )
    .await
    .map_err_response(ErrorResponse::InternalServerError)?;

    let mailer = build_mailer(state)
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;
    let values = EmailValues {
        first_name: event.donor.first_name.clone(),
        last_name: event.donor.last_name.clone(),
        invite_url,
        email: event.donor.email.clone(),
        timestamp: event.donation_date.to_string(),
        amount_paid: event.formatted_net_amount.clone(),
        donor_id: event.donor.id.to_string(),
        donor_url: format!(
            "https://donorbox.org/org_admin/supporters/{}",
            event.donor.id
        ),
        donation_id: event.id.to_string(),
        donation_url: format!("https://donorbox.org/org_admin/donations/{}", event.id),
        plan_id: event.plan_id.to_string(),
        plan_url: format!("https://donorbox.org/org_admin/plans/{}", event.plan_id),
        payment_id: event.stripe_charge_id.clone(),
        payment_url: format!(
            "https://dashboard.stripe.com/payments/{}",
            event.stripe_charge_id
        ),
        referral_source: event.questions.get(0).cloned().unwrap_or_default().answer,
    };

    let board_notif_address = state
        .persist
        .load::<String>("board_notif_address")
        .map_err_response(ErrorResponse::InternalServerError)?;
    let board_notif_future = mailer.send(
        build_message(
            "board_notif",
            "New Member Notification",
            &board_notif_address,
            &values,
            state,
        )
        .map_err_response(ErrorResponse::InternalServerError)?,
    );

    let discord_future = mailer.send(
        build_message(
            "discord",
            "Psychedelic Club Discord",
            &event.donor.email,
            &values,
            state,
        )
        .map_err_response(ErrorResponse::InternalServerError)?,
    );

    try_join!(discord_future, board_notif_future)
        .map_err_response(ErrorResponse::InternalServerError)?;
    Ok(())
}

pub async fn webhook_handler(
    State(state): State<crate::AppState>,
    Json([event]): Json<[DonationEvent; 1]>,
) -> Result<Json<ResponseBody>, Response> {
    if event.action != "new" {
        return Err("action != 'new'")
            .map_err_response(ErrorResponse::StatusCode(StatusCode::NO_CONTENT));
    }
    if event.campaign.id
        != state
            .secret_store
            .get("DONORBOX_CAMPAIGN_ID")
            .unwrap()
            .parse::<i32>()
            .unwrap()
    {
        return Err("wrong campaign")
            .map_err_response(ErrorResponse::StatusCode(StatusCode::NO_CONTENT));
    }

    let created_member_id = sqlx::query_scalar!(
        "INSERT INTO members (email, first_name, last_name)
        SELECT $1, $2, $3
        WHERE NOT EXISTS (SELECT * FROM members WHERE email = $1)
        ON CONFLICT DO NOTHING
        RETURNING id",
        event.donor.email,
        event.donor.first_name,
        event.donor.last_name
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err_response(ErrorResponse::InternalServerError)?;

    let inserted_transaction = sqlx::query_as!(
        InsertTransactionResult,
        r#"INSERT INTO payments (member_id, amount_paid, payment_method, transaction_id, effective_on)
            SELECT               id,        $2,          'donorbox',     $3,             $4
            FROM members
            WHERE email = $1
        RETURNING id, member_id"#,
        event.donor.email,
        event.net_amount,
        event.id,
        event.donation_date.date()
    ).fetch_one(&state.db_pool).await.map_err_response(ErrorResponse::InternalServerError)?;

    if created_member_id.is_some() {
        send_emails(&state, &event).await?;
    }

    Ok(Json(ResponseBody {
        created_member_id,
        inserted_transaction,
    }))
}
