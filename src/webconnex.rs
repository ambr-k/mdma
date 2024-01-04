mod new_member;
mod recurring_payment_success;
mod verify;
use self::verify::{ver_sig, VerifySigState};

use askama_axum::IntoResponse;
use axum::{
    http::StatusCode, middleware::from_fn_with_state, response::Response, routing::post, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestPayload {
    data: EventDetails,
}

#[derive(Deserialize)]
struct Name {
    first: String,
    last: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Billing {
    payment_method: String,
    email: String,
    name: Name,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventDetails {
    customer_id: i32,
    total: Decimal,
    billing: Billing,
    lookup_id: i32,
    transaction_id: i32,
}

struct InsertTransactionResponse {
    id: i32,
    member_id: i32,
}

async fn insert_transaction(
    body: &EventDetails,
    state: &crate::AppState,
) -> Result<InsertTransactionResponse, Response> {
    sqlx::query_as!(
        InsertTransactionResponse,
        r#"INSERT INTO payments (member_id, amount_paid, method, platform, subscription_id, transaction_id)
            SELECT id, $2, $3, 'webconnex', $4, $5
            FROM members
            WHERE webconnex_id = $1
        RETURNING id, member_id"#,
        body.customer_id,
        body.total,
        body.billing.payment_method,
        body.lookup_id,
        body.transaction_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}

pub fn router(state: crate::AppState) -> Router {
    let new_member_ver_state = VerifySigState {
        hmac_secret: state
            .secret_store
            .get("WC_NEWMEMBER_HMAC")
            .expect("Couldn't find secret WC_NEWMEMBER_HMAC"),
    };

    let recurring_success_ver_state = VerifySigState {
        hmac_secret: state
            .secret_store
            .get("WC_RECURRINGSUCCESS_HMAC")
            .expect("Couldn't find secret WC_RECURRINGSUCCESS_HMAC"),
    };

    Router::new()
        .route(
            "/new-member",
            post(new_member::webhook_handler)
                .route_layer(from_fn_with_state(new_member_ver_state, ver_sig)),
        )
        .route(
            "/payment-success",
            post(recurring_payment_success::webhook_handler)
                .route_layer(from_fn_with_state(recurring_success_ver_state, ver_sig)),
        )
        .with_state(state)
}
