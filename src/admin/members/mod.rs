use axum::{routing::get, Router};

mod details;
mod new_payment;
mod search;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(search::members_list))
        .route("/search", get(search::search_results))
        .route("/details/:member_id", get(details::details))
        .route(
            "/new_payment/:member_id",
            get(new_payment::payment_form).post(new_payment::add_payment),
        )
        .with_state(state.clone())
}
