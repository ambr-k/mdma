use super::{insert_transaction, RequestPayload};
use askama_axum::IntoResponse;
use axum::{extract::State, http::StatusCode, response::Response, Json};

struct SqlCreateResponse {
    id: i32,
}

async fn create_user(
    body: &RequestPayload,
    state: &crate::AppState,
) -> Result<SqlCreateResponse, Response> {
    sqlx::query_as!(
        SqlCreateResponse,
        r#"INSERT INTO members (webconnex_id, first_name, last_name, email)
        VALUES ($1, $2, $3, $4)
        RETURNING id"#,
        body.customer_id,
        body.billing.name.first,
        body.billing.name.last,
        body.billing.email
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
    Json(body): Json<RequestPayload>,
) -> Result<axum::Json<ResponseBody>, Response> {
    let create_response = create_user(&body, &state).await?;
    let insert_response = insert_transaction(&body, &state).await?;

    Ok(Json(ResponseBody {
        created_member_id: create_response.id,
        transaction_id: insert_response.id,
        transaction_member_id: insert_response.member_id,
    }))
}
