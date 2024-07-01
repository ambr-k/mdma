mod auth;
mod db_create_user;
mod db_insert_transaction;
mod new_member;
mod recurring_payment_success;
mod request_payload;

use axum::{middleware::from_fn_with_state, routing::post, Router};

use self::auth::{ver_sig, VerifySigState};

pub fn router(state: crate::AppState) -> Router {
    let new_member_ver_state = VerifySigState {
        hmac_secret: state
            .secret_store
            .get("WC_NEWMEMBER_HMAC")
            .expect("Couldn't find secret WC_NEWMEMBER_HMAC"),
    };

    let recurring_success_ver_state = VerifySigState {
        hmac_secret: state
            .secret_store
            .get("WC_RECURRINGSUCCESS_HMAC")
            .expect("Couldn't find secret WC_RECURRINGSUCCESS_HMAC"),
    };

    Router::new()
        .route(
            "/new-member",
            post(new_member::webhook_handler)
                .route_layer(from_fn_with_state(new_member_ver_state, ver_sig)),
        )
        .route(
            "/payment-success",
            post(recurring_payment_success::webhook_handler)
                .route_layer(from_fn_with_state(recurring_success_ver_state, ver_sig)),
        )
        .with_state(state)
}
