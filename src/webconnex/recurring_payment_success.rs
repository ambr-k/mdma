use super::{insert_transaction, RequestPayload};
use axum::{extract::State, response::Response, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ResponseBody {
    member_id: i32,
    transaction_id: i32,
}

pub async fn webhook_handler(
    State(state): State<crate::AppState>,
    Json(RequestPayload { data: event }): Json<RequestPayload>,
) -> Result<axum::Json<ResponseBody>, Response> {
    let insert_response = insert_transaction(&event, &state).await?;

    Ok(Json(ResponseBody {
        member_id: insert_response.member_id,
        transaction_id: insert_response.id,
    }))
}
