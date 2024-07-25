use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use lettre::AsyncTransport;
use reqwest::StatusCode;
use serenity::all::ChannelId;

use crate::send_email::{build_mailer, build_message, EmailValues};

use super::{
    db_create_user::{create_user, SqlCreateResponse},
    db_insert_transaction::{insert_transaction, InsertTransactionResponse},
    request_payload::{EventDetails, RequestPayload},
};

#[derive(serde::Serialize)]
struct InviteOptions {
    max_age: u64,
    max_uses: u8,
    unique: bool,
}

impl Default for InviteOptions {
    fn default() -> Self {
        Self {
            max_age: 604800,
            max_uses: 1,
            unique: true,
        }
    }
}

async fn send_emails(state: &crate::AppState, event: &EventDetails) -> Result<(), Response> {
    let invite_url = state
        .discord_http
        .create_invite(
            state
                .secret_store
                .get("DISCORD_INVITE_CHANNEL_ID")
                .unwrap()
                .parse::<ChannelId>()
                .unwrap(),
            &InviteOptions::default(),
            Some(&format!(
                "New member automated invite (GivingFuel order #{}, Email {})",
                event.transaction_id, event.billing.email
            )),
        )
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .url();

    let mailer = build_mailer(state)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;
    let values = EmailValues {
        first_name: event.billing.name.first.clone(),
        invite_url,
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
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err).into_response())?,
        )
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;
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
