use crate::{
    db::members::{MemberRow, MembersQuery},
    icons,
};
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
use tokio::try_join;

struct SelectIdOption {
    id: i32,
    description: String,
}

pub struct PaginationRequest {
    count: u64,
    offset: u64,
}

pub async fn users_list(
    Query(params): Query<MembersQuery>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let (members, total) = try_join!(
        crate::db::members::search(&params, &state),
        crate::db::members::count(&params, &state)
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let generation_options = sqlx::query_as!(
        SelectIdOption,
        r#"SELECT id, CONCAT(title, ' (', start_date, ')') AS "description!" FROM generations"#
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    let pagebtn = |request_opt: Option<PaginationRequest>, text: &str| -> Markup {
        html! {
            @if let Some(request) = request_opt {
                button ."btn"."btn-outline"."join-item"."w-1/4" hx-get="admin/users" hx-target="#members-list" hx-swap="outerHTML"
                    hx-vals=(serde_json::to_string(&MembersQuery {count: request.count, offset: request.offset, ..params.clone()}).unwrap()) {(text)}
            }
            @else { button ."btn"."btn-outline"."join-item"."w-1/4" disabled {(text)}}
        }
    };

    let prev = match params.offset {
        0 => None,
        _ => Some(PaginationRequest {
            count: params.count,
            offset: std::cmp::max(params.offset - params.count, 0),
        }),
    };
    let next = if params.offset + params.count >= total {
        None
    } else {
        Some(PaginationRequest {
            count: params.count,
            offset: params.offset + params.count,
        })
    };

    Ok(html! { #"members-list" ."w-full"."max-w-xl"."mx-auto" {
        #"members-search" ."collapse"."collapse-arrow"."bg-base-200"."my-2"."border"."border-secondary" {
            input type="radio" name="members-list-accordion" checked;
            ."collapse-title"."text-xl"."font-medium" {"Search Members"}
            ."collapse-content" {
                form ."[&>*]:my-3" hx-get="admin/users" hx-swap="outerHTML" hx-target="#members-list" {

                    label ."input"."input-bordered"."flex"."items-center"."gap-2" {
                        input type="text" name="search" placeholder="Search" value=[&params.search] ."grow"."bg-inherit";
                        span ."text-secondary" {(icons::search())}
                    }
                    label ."input"."input-bordered"."flex"."items-center"."gap-3" {
                        (icons::discord())
                        input type="text" name="discord" placeholder="Discord" value=[&params.discord] ."grow"."bg-inherit";
                    }
                    ."form-control" {
                        label ."label"."cursor-pointer" {
                            span ."label-text" {"Active Members Only"}
                            input type="checkbox" name="active_only" value="true" checked[params.active_only] ."checkbox"."checkbox-primary";
                        }
                    }
                    ."form-control" {
                        label ."label"."cursor-pointer" {
                            span ."label-text" {"Generation"}
                            select name="generation_id" ."select"."select-bordered" {
                                option value="-1" {"(Any Generation)"}
                                @for gen in generation_options {
                                    option value=(gen.id) selected[gen.id==params.generation_id] {(gen.description)}
                                }
                            }
                        }
                    }
                    ."divider" {"Sort Results"}
                    ."form-control" {
                        label ."label"."cursor-pointer" {
                            span ."label-text" {"Sort By"}
                            select name="sort_by" ."select"."select-bordered" {
                                option value="" {"(Default)"}
                                option value="firstname" selected[params.sort_by=="firstname"] {"First Name"}
                                option value="lastname" selected[params.sort_by=="lastname"] {"Last Name"}
                                option value="consecutivesince" selected[params.sort_by=="consecutivesince"] {"Active Since"}
                            }
                        }
                    }
                    ."form-control" {
                        label ."label"."cursor-pointer" {
                            span ."label-text" {"Descending"}
                            input type="checkbox" name="sort_desc" value="true" checked[params.sort_desc] ."checkbox"."checkbox-primary";
                        }
                    }
                    button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0" {"SEARCH"}

                }
            }
        }
        ."divider" {}
        @for member in &members {
            ."collapse"."collapse-arrow"."bg-base-200"."my-2" {
                input #{"user_details_trigger_"(member.id)} type="radio" name="members-list-accordion"
                    hx-get={"admin/user/"(member.id)} hx-target="next .collapse-content" hx-swap="innerHTML" hx-indicator="closest .collapse";
                ."collapse-title"."text-xl"."font-medium" {(member.last_name)", "(member.first_name)}
                #{"user_details_"(member.id)} ."collapse-content" { progress ."progress"."htmx-indicator" {} }
            }
        }
        ."divider" {}
        #"members-pagination" ."join"."join-vertical"."md:join-horizontal"."justify-center"."w-full"."items-center" {
            (pagebtn(prev, "Previous"))
            ."btn"."btn-outline"."join-item"."w-1/4"."!text-neutral-content" disabled {
                (params.offset + 1)" - "(params.offset + (members.len() as u64))" of "(total)
            }
            (pagebtn(next, "Next"))
        }
    }})
}

pub enum DiscordMembership {
    GuildMember(serenity::model::guild::Member),
    GlobalUser(serenity::model::user::User),
}

#[derive(Template)]
#[template(path = "admin/user_details.html")]
pub struct UserDetailsTemplate {
    user: MemberRow,
    webconnex: WebconnexCustomerSearchResponse,
    discord: Option<DiscordMembership>,
    discord_role: Option<serenity::model::guild::Role>,
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
        MemberRow,
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

    let discord_role = match &discord {
        Some(DiscordMembership::GuildMember(member)) => {
            match state
                .discord_http
                .get_guild_roles(state.discord_guild)
                .await
            {
                Err(_) => None,
                Ok(roles) => roles
                    .iter()
                    .filter(|r| member.roles.contains(&r.id))
                    .max_by_key(|r| r.position)
                    .map(ToOwned::to_owned),
            }
        }
        _ => None,
    };

    Ok(UserDetailsTemplate {
        user,
        webconnex,
        discord,
        discord_role,
    })
}

#[derive(Template)]
#[template(path = "admin/user_add_payment.html")]
pub struct NewPaymentFormTemplate {
    user: MemberRow,
}

pub async fn user_payment_form(
    Path(user_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<NewPaymentFormTemplate, Response> {
    let user = sqlx::query_as!(
        MemberRow,
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
