use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::request_payload;

pub struct InsertTransactionResponse {
    pub id: i32,
    pub member_id: i32,
}

pub async fn insert_transaction(
    body: &request_payload::EventDetails,
    state: &crate::AppState,
) -> Result<InsertTransactionResponse, Response> {
    sqlx::query_as!(
        InsertTransactionResponse,
        r#"INSERT INTO payments (member_id, amount_paid, payment_method, platform, transaction_id)
            SELECT id, $2, $3, 'webconnex', $4
            FROM members
            WHERE email = $1
        RETURNING id, member_id"#,
        body.billing.email.to_lowercase(),
        body.total,
        body.billing.payment_method,
        body.transaction_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}
