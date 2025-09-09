use axum::{middleware::from_fn_with_state, routing::post, Router};

mod auth;
pub mod new_donation;

#[derive(serde::Deserialize)]
pub struct Donor {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/new-donation", post(new_donation::webhook_handler))
        .route_layer(from_fn_with_state(
            state.secret_store.get("DONORBOX_HMAC").unwrap(),
            auth::ver_sig,
        ))
        .with_state(state.clone())
}
