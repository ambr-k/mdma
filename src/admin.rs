use askama::Template;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};

mod bulk_update;
mod users;

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminRootTemplate<'a> {
    title: &'a str,
}

async fn root() -> AdminRootTemplate<'static> {
    AdminRootTemplate { title: "Admin" }
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/users", get(users::users_list))
        .route("/user/:user_id", get(users::user_details))
        .route("/user/:user_id/payment", post(users::add_payment))
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
