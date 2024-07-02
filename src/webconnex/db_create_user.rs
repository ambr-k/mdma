use askama_axum::IntoResponse;
use axum::{http::StatusCode, response::Response};

use super::request_payload;

pub struct SqlCreateResponse {
    pub id: i32,
}

pub async fn create_user(
    event: &request_payload::EventDetails,
    state: &crate::AppState,
) -> Result<SqlCreateResponse, Response> {
    sqlx::query_as!(
        SqlCreateResponse,
        r#"INSERT INTO members (email, first_name, last_name)
        VALUES ($1, $2, $3)
        RETURNING id"#,
        event.billing.email.to_lowercase(),
        event.billing.name.first,
        event.billing.name.last
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}
