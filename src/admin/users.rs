use crate::icons;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::Response,
};
use maud::{html, Markup, PreEscaped};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use sqlx::Error;
use time::Date;

#[allow(dead_code)]
struct User {
    id: i32,
    email: String,
    first_name: String,
    last_name: String,
    reason_removed: Option<String>,
    created_on: Date,
    consecutive_since_cached: Option<Date>,
    consecutive_until_cached: Option<Date>,
    generation_name: Option<String>,
    discord: Option<Decimal>,
}

struct SelectIdOption {
    id: i32,
    description: String,
}

pub struct PaginationRequest {
    count: i64,
    offset: i64,
}

#[derive(Template)]
#[template(path = "admin/users_list.html")]
pub struct UsersListTemplate {
    users: Vec<User>,
    search: Option<String>,
    active_only: bool,
    generation_options: Vec<SelectIdOption>,
    generation_id: i32,
    range: (i64, i64, i64),

    prev: Option<PaginationRequest>,
    next: Option<PaginationRequest>,
}

#[serde_inline_default]
#[derive(Deserialize)]
pub struct UsersListQuery {
    search: Option<String>,

    #[serde_inline_default(false)]
    active_only: bool,

    #[serde_inline_default(12)]
    count: i64,

    #[serde_inline_default(0)]
    offset: i64,

    #[serde_inline_default(-1)]
    generation_id: i32,
}

pub async fn users_list(
    Query(params): Query<UsersListQuery>,
    State(state): State<crate::AppState>,
) -> UsersListTemplate {
    let users = sqlx::query_as!(
        User,
        r#"
            SELECT members.*, generations.title AS generation_name
            FROM members
                LEFT JOIN member_generations ON members.id = member_id
                LEFT JOIN generations ON generations.id = generation_id
            WHERE (
                $1::text IS NULL
                OR POSITION(LOWER($1::text) IN LOWER(first_name || ' ' || last_name)) > 0
                OR POSITION(LOWER($1::text) IN LOWER(email)) > 0
            ) AND (
                NOT $4
                OR (consecutive_until_cached > NOW() AND reason_removed IS NULL)
            ) AND (
                $5::int < 0
                OR generations.id = $5::int
            )
            LIMIT $2 OFFSET $3"#,
        params.search,
        params.count,
        params.offset,
        params.active_only,
        params.generation_id
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    let total = sqlx::query_scalar!(
        r#"
            SELECT COUNT(*)
            FROM members
                LEFT JOIN member_generations ON members.id = member_id
            WHERE (
                $1::text IS NULL
                OR POSITION(LOWER($1::text) IN LOWER(first_name || ' ' || last_name)) > 0
                OR POSITION(LOWER($1::text) IN LOWER(email)) > 0
            ) AND (
                NOT $2 OR consecutive_until_cached > NOW()
            ) AND (
                $3::int < 0
                OR generation_id = $3::int
            )
            "#,
        params.search,
        params.active_only,
        params.generation_id
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap()
    .unwrap_or_default();

    let generation_options = sqlx::query_as!(
        SelectIdOption,
        r#"SELECT id, CONCAT(title, ' (', start_date, ')') AS "description!" FROM generations"#
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    UsersListTemplate {
        search: params.search,
        active_only: params.active_only,
        generation_id: params.generation_id,
        generation_options,
        range: (
            params.offset + 1,
            params.offset + (users.len() as i64),
            total,
        ),
        users,
        prev: match params.offset {
            0 => None,
            _ => Some(PaginationRequest {
                count: params.count,
                offset: std::cmp::max(params.offset - params.count, 0),
            }),
        },
        next: if params.offset + params.count >= total {
            None
        } else {
            Some(PaginationRequest {
                count: params.count,
                offset: params.offset + params.count,
            })
        },
    }
}

pub enum DiscordMembership {
    GuildMember(serenity::model::guild::Member),
    GlobalUser(serenity::model::user::User),
}

#[derive(Template)]
#[template(path = "admin/user_details.html")]
pub struct UserDetailsTemplate {
    user: User,
    webconnex: WebconnexCustomerSearchResponse,
    discord: Option<DiscordMembership>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebconnexCustomerData {
    id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebconnexCustomerSearchResponse {
    data: Option<Vec<WebconnexCustomerData>>,
}

pub async fn user_details(
    Path(user_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<UserDetailsTemplate, Response> {
    let user = sqlx::query_as!(
        User,
        r#" SELECT members.*, generations.title AS generation_name
            FROM members
                INNER JOIN member_generations ON members.id = member_id
                INNER JOIN generations ON generations.id = generation_id
            WHERE members.id=$1"#,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let webconnex = state
        .http_client
        .get("https://api.webconnex.com/v2/public/search/customers")
        .query(&[("product", "givingfuel.com"), ("orderEmail", &user.email)])
        .header(
            "apiKey",
            state.secret_store.get("WEBCONNEX_API_KEY").unwrap(),
        )
        .send()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .json::<WebconnexCustomerSearchResponse>()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let discord = match user.discord.and_then(|uid| uid.to_u64()) {
        None => None,
        Some(uid) => {
            let user_id = serenity::model::id::UserId::new(uid);
            match state
                .discord_http
                .get_member(state.discord_guild, user_id)
                .await
            {
                Ok(member) => Some(DiscordMembership::GuildMember(member)),
                Err(_) => match state.discord_http.get_user(user_id).await {
                    Ok(user) => Some(DiscordMembership::GlobalUser(user)),
                    Err(_) => None,
                },
            }
        }
    };

    Ok(UserDetailsTemplate {
        user,
        webconnex,
        discord,
    })
}

#[derive(Template)]
#[template(path = "admin/user_add_payment.html")]
pub struct NewPaymentFormTemplate {
    user: User,
}

pub async fn user_payment_form(
    Path(user_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<NewPaymentFormTemplate, Response> {
    let user = sqlx::query_as!(
        User,
        r#" SELECT members.*, generations.title AS generation_name
            FROM members
                INNER JOIN member_generations ON members.id = member_id
                INNER JOIN generations ON generations.id = generation_id
            WHERE members.id=$1"#,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    Ok(NewPaymentFormTemplate { user })
}

#[serde_inline_default]
#[derive(Deserialize)]
pub struct NewPaymentFormData {
    payment_method: String,
    amount_paid: Option<Decimal>,
    transaction_id: Option<i32>,
    effective_on: Date,
    duration_months: i32,
    notes: Option<String>,
}

pub async fn add_payment(
    State(state): State<crate::AppState>,
    Path(user_id): Path<i32>,
    Form(form): Form<NewPaymentFormData>,
) -> Markup {
    let sql_result = sqlx::query_scalar!(
        r#"INSERT INTO payments (member_id, effective_on, duration_months, amount_paid, payment_method, platform, transaction_id, notes)
            VALUES              ($1,        $2,           $3,              $4,          $5,             'mdma',   $6,             $7)
            RETURNING id"#,
        user_id,
        form.effective_on,
        form.duration_months,
        form.amount_paid.unwrap_or(Decimal::ZERO),
        form.payment_method,
        form.transaction_id,
        form.notes
    ).fetch_one(&state.db_pool)
    .await;

    match sql_result {
        Ok(transaction_id) => html! {
            div hx-swap-oob={"innerHTML:#user_details_"(user_id)} {
                progress ."progress"."htmx-indicator" {
                    script {(PreEscaped(format!("
                        $('#modal')[0].close();
                        htmx.trigger('#user_details_trigger_{}', 'change', {{}});
                    ", user_id)))}
                }
            }
            div hx-swap-oob="beforeend:#alerts" {
                #{"alert_payment_success_"(transaction_id)}."alert"."alert-success"."transition-opacity"."duraiton-300" role="alert" {
                    (icons::success())
                    span {"Payment Added Successfully"}
                    script {(PreEscaped(format!("
                        setTimeout(() => {{
                            const toastElem = $('#alert_payment_success_{}');
                            toastElem.on('transitionend', (event) => {{event.target.remove();}});
                            toastElem.css('opacity', 0);
                        }}, 2500);
                    ", transaction_id)))}
                }
            }
        },
        Err(err) => html! {
            ."alert"."alert-error" role="alert" {
                (icons::error())
                span {(err.to_string())}
            }
        },
    }
}
