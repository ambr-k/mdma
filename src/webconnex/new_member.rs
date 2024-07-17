use axum::{extract::State, response::Response, Json};

use super::{
    db_create_user::{create_user, SqlCreateResponse},
    db_insert_transaction::{insert_transaction, InsertTransactionResponse},
    request_payload::RequestPayload,
};

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

    Ok(Json(ResponseBody {
        create_user: create_response.ok(),
        insert_transaction: insert_response,
    }))
}
