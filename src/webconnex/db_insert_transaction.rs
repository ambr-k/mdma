use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::err_responses::{ErrorResponse, MapErrorResponse};

use super::request_payload;

#[derive(serde::Serialize)]
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
        r#"INSERT INTO payments (member_id, amount_paid, payment_method, transaction_id)
            SELECT               id,        $2,          'webconnex',    $3
            FROM members
            WHERE email = $1
        RETURNING id, member_id"#,
        body.billing.email.to_lowercase(),
        body.total,
        body.transaction_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err_response(ErrorResponse::InternalServerError)
}
