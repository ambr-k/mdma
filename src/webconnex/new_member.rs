use axum::{extract::State, response::Response, Json};

use super::{
    db_create_user::create_user, db_insert_transaction::insert_transaction,
    request_payload::RequestPayload,
};

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
