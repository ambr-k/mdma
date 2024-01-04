use askama::Template;
use axum::{middleware, routing::get, Router};

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
        .route("/users/list.hx", get(users::users_list))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::verify_admin,
        ))
}
