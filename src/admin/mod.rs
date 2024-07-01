use axum::{
    middleware,
    routing::{get, post},
    Router,
};

mod bulk_update;
mod generations;
mod users;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/users", get(users::users_list))
        .route("/user/:user_id", get(users::user_details))
        .route(
            "/user/:user_id/payment",
            post(users::add_payment).get(users::user_payment_form),
        )
        .route("/generations", get(generations::generations_list))
        .route("/bulk_update", get(bulk_update::bulk_update_form))
        .route(
            "/.givingfuel_bulk_import",
            post(bulk_update::submit_givingfuel_bulk_update),
        )
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::verify_admin,
        ))
}
