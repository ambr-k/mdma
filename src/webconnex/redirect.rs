use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use reqwest::StatusCode;
use serde::Deserialize;

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
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .json::<WebconnexTransactionResponse>()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
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
