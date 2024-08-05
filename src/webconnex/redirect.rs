use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::err_responses::{ErrorResponse, MapErrorResponse};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebconnexTransactionData {
    order_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebconnexTransactionResponse {
    data: WebconnexTransactionData,
}

pub async fn transaction(
    Path(txid): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Redirect, Response> {
    let txobj = state
        .http_client
        .get(format!(
            "https://api.webconnex.com/v2/public/search/transactions/{txid}?product=givingfuel.com"
        ))
        .header(
            "apiKey",
            state.secret_store.get("WEBCONNEX_API_KEY").unwrap(),
        )
        .send()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?
        .json::<WebconnexTransactionResponse>()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?
        .data;

    Ok(Redirect::permanent(&format!(
        "https://manage.webconnex.com/reports/orders/{}/donations/{}",
        txobj.order_id, txid
    )))
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/transaction/:txid", get(transaction))
        .with_state(state.clone())
}
