use askama::Template;
use axum::extract::State;
use time::Date;

#[allow(dead_code)]
struct User {
    id: i32,
    webconnex_id: Option<i32>,
    first_name: String,
    last_name: String,
    email: String,
    reason_removed: Option<String>,
    created_on: Date,
}

#[derive(Template)]
#[template(path = "admin/users_list.html")]
pub struct UsersListTemplate {
    users: Vec<User>,
}

pub async fn users_list(State(state): State<crate::AppState>) -> UsersListTemplate {
    let users = sqlx::query_as!(User, "SELECT * FROM members")
        .fetch_all(&state.db_pool)
        .await
        .unwrap();

    UsersListTemplate { users }
}
