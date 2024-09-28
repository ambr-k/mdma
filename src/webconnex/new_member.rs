use axum::{extract::State, response::Response, Json};
use lettre::AsyncTransport;

use crate::{
    discord::create_invite,
    err_responses::{ErrorResponse, MapErrorResponse},
    send_email::{build_mailer, build_message, EmailValues},
};

use super::{
    db_create_user::{create_user, SqlCreateResponse},
    db_insert_transaction::{insert_transaction, InsertTransactionResponse},
    request_payload::{EventDetails, RequestPayload},
};

async fn send_emails(state: &crate::AppState, event: &EventDetails) -> Result<(), Response> {
    let invite_url = create_invite(
        Some(&format!(
            "New member automated invite (GivingFuel order #{}, Email {})",
            event.transaction_id, event.billing.email
        )),
        state,
    )
    .await
    .map_err_response(ErrorResponse::InternalServerError)?;

    let mailer = build_mailer(state)
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;
    let values = EmailValues {
        first_name: event.billing.name.first.clone(),
        last_name: event.billing.name.last.clone(),
        invite_url,
        email: event.billing.email.clone(),
        ..Default::default()
    };

    mailer
        .send(
            build_message(
                "discord",
                "Psychedelic Club Discord",
                &event.billing.email,
                &values,
                state,
            )
            .map_err_response(ErrorResponse::InternalServerError)?,
        )
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct ResponseBody {
    create_user: Option<SqlCreateResponse>,
    insert_transaction: InsertTransactionResponse,
}

pub async fn webhook_handler(
    State(state): State<crate::AppState>,
    Json(RequestPayload { data: event }): Json<RequestPayload>,
) -> Result<axum::Json<ResponseBody>, Response> {
    let create_response = create_user(&event, &state).await;
    let insert_response = insert_transaction(&event, &state).await?;

    send_emails(&state, &event).await?;

    Ok(Json(ResponseBody {
        create_user: create_response.ok(),
        insert_transaction: insert_response,
    }))
}
