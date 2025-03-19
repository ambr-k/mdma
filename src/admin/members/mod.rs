use axum::{
    routing::{get, post},
    Router,
};

mod cancel_ban;
mod create_member;
mod details;
mod new_payment;
mod search;

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(search::members_list))
        .route("/search", get(search::search_results))
        .route("/details/{member_id}", get(details::details))
        .route(
            "/send_discord_email/{member_id}",
            post(details::send_discord_email),
        )
        .route(
            "/new_payment/{member_id}",
            get(new_payment::payment_form).post(new_payment::add_payment),
        )
        .route(
            "/create",
            get(create_member::member_form).post(create_member::add_member),
        )
        .route(
            "/cancel/{member_id}",
            get(cancel_ban::cancel_form).post(cancel_ban::cancel_member),
        )
        .route(
            "/ban/{member_id}",
            get(cancel_ban::ban_form).post(cancel_ban::ban_member),
        )
        .route(
            "/unban/{member_id}",
            get(cancel_ban::unban_form).post(cancel_ban::unban_member),
        )
        .with_state(state.clone())
}
