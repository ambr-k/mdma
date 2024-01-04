use super::{insert_transaction, EventDetails, RequestPayload};
use askama_axum::IntoResponse;
use axum::{extract::State, http::StatusCode, response::Response, Json};

struct SqlCreateResponse {
    id: i32,
}

async fn create_user(
    event: &EventDetails,
    state: &crate::AppState,
) -> Result<SqlCreateResponse, Response> {
    sqlx::query_as!(
        SqlCreateResponse,
        r#"INSERT INTO members (webconnex_id, first_name, last_name, email)
        VALUES ($1, $2, $3, $4)
        RETURNING id"#,
        event.customer_id,
        event.billing.name.first,
        event.billing.name.last,
        event.billing.email
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}

#[derive(serde::Serialize)]
pub struct ResponseBody {
    created_member_id: i32,
    transaction_id: i32,
    transaction_member_id: i32,
}

pub async fn webhook_handler(
    State(state): State<crate::AppState>,
    Json(RequestPayload { data: event }): Json<RequestPayload>,
) -> Result<axum::Json<ResponseBody>, Response> {
    let create_response = create_user(&event, &state).await?;
    let insert_response = insert_transaction(&event, &state).await?;

    Ok(Json(ResponseBody {
        created_member_id: create_response.id,
        transaction_id: insert_response.id,
        transaction_member_id: insert_response.member_id,
    }))
}
