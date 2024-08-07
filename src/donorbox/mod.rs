use axum::{middleware::from_fn_with_state, routing::post, Router};

mod auth;
mod new_donation;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/new-donation", post(new_donation::webhook_handler))
        .route_layer(from_fn_with_state(
            state.secret_store.get("DONORBOX_HMAC").unwrap(),
            auth::ver_sig,
        ))
        .with_state(state.clone())
}
