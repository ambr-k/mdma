use axum::{routing::get, Router};

mod search;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(search::search_form))
        .route("/search", get(search::search_results))
        .with_state(state.clone())
}
