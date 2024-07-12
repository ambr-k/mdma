use axum::{routing::get, Router};

mod details;
mod search;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(search::members_list))
        .route("/search", get(search::search_results))
        .route("/details/:member_id", get(details::details))
        .with_state(state.clone())
}
