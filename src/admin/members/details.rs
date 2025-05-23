use axum::{
    extract::{NestedPath, Path, State},
    response::{IntoResponse, Response},
};
use lettre::AsyncTransport;
use maud::{html, Markup};
use reqwest::StatusCode;
use rust_decimal::prelude::ToPrimitive;
use serde::Deserialize;

use crate::{
    components,
    db::members::MemberDetailsRow,
    discord::create_invite,
    err_responses::{ErrorResponse, MapErrorResponse},
    icons,
    send_email::{build_mailer, build_message, EmailValues},
};

pub enum DiscordMembership {
    GuildMember(serenity::model::guild::Member),
    GlobalUser(serenity::model::user::User),
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

pub async fn details(
    nest: NestedPath,
    Path(member_id): Path<i32>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    let member = sqlx::query_as!(
        MemberDetailsRow,
        "SELECT * FROM members NATURAL JOIN member_details WHERE id=$1",
        member_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    let webconnex = state
        .http_client
        .get("https://api.webconnex.com/v2/public/search/customers")
        .query(&[("product", "givingfuel.com"), ("orderEmail", &member.email)])
        .header(
            "apiKey",
            state.secret_store.get("WEBCONNEX_API_KEY").unwrap(),
        )
        .send()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?
        .json::<WebconnexCustomerSearchResponse>()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?
        .data
        .unwrap_or_default();

    let donorbox = state
        .http_client
        .get("https://donorbox.org/api/v1/donors")
        .basic_auth(
            state.secret_store.get("DONORBOX_APILOGIN").unwrap(),
            state.secret_store.get("DONORBOX_APIKEY"),
        )
        .query(&[("email", &member.email)])
        .send()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?
        .json::<Vec<crate::donorbox::Donor>>()
        .await
        .map_err_response(ErrorResponse::InternalServerError)?;

    let discord_opt = match member.discord.and_then(|uid| uid.to_u64()) {
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

    let discord_role = match &discord_opt {
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

    Ok(html! {
        ."divider" {"Member Details"}
        ."*:mx-1" {
            @if member.banned { ."badge"."badge-error" {"Banned"} }
            @else if member.cancelled { ."badge"."badge-warning" {"Cancelled"} }
            @if member.is_active == Some(true) { ."badge"."badge-success"."badge-outline" {"Active"} }
            @else { ."badge".{"badge-"(if member.cancelled || member.banned {"outline"} else {"info"})} {"Inactive"} }
            // ."badge-info"
        }
        a ."btn"."btn-link" href={"mailto:"(member.email)} {(member.email)}
        @match member.first_payment {
            None => p {"No recorded payments"},
            Some(first_payment) => {
                p {"First joined on "(first_payment)}
                @if let Some(val) = member.consecutive_since {
                    p {"Active since "(val)" ("(member.generation_name.as_deref().unwrap_or("<null>"))" Generation)"}
                }
                @if let Some(val) = member.consecutive_until {
                    p {"Active until "(val)}
                }
            }
        }
        ."divider" {"Third-Party Accounts"}
        ."*:mr-2" {
            @for wc_account in webconnex {
                a ."btn"."btn-outline"."btn-primary" href={"https://manage.webconnex.com/contacts/"(wc_account.id)} target="_blank" {"GivingFuel ID "(wc_account.id)}
            }
            @for db_account in donorbox {
                a ."btn"."btn-outline"."btn-primary" href={"https://donorbox.org/org_admin/supporters/"(db_account.id)} target="_blank" {"Donorbox ID "(db_account.id)}
            }
        }
        @if let Some(discord) = discord_opt {
            ."card"."card-compact"."card-side"."bg-neutral"."h-24"."w-full"."max-w-sm"."mx-auto"."my-2" {
                @match discord {
                    DiscordMembership::GuildMember(discord_member) => {
                        figure ."w-24" { img src=(discord_member.face()); }
                        ."card-body" {
                            ."card-title"."!mb-0" {
                                (icons::discord())
                                @if let Some(role) = discord_role {
                                    ."badge"."badge-outline" style={"border-color:#"(role.colour.hex())"; color:#"(role.colour.hex())";"} {(role.name)}
                                } @else { ."badge"."badge-warning" {"Guild Member, No Role"} }
                            }
                            p { b {(discord_member.display_name())} br; i {"("(discord_member.user.name)")"} }
                        }
                    },
                    DiscordMembership::GlobalUser(discord_user) => {
                        figure ."w-24" { img src=(discord_user.face()); }
                        ."card-body" {
                            ."card-title" {
                                (icons::discord())
                                ."badge"."badge-error" {"Not a Guild Member"}
                            }
                            p {(discord_user.name)}
                        }
                    }
                }
            }
        }
        @if !member.notes.is_empty() {
            ."divider" {"Notes"}
            pre ."w-full"."overflow-x-auto" {(member.notes.trim())}
        }
        ."divider"."mb-0" {"Actions"}
        ."*:mt-3"."*:mr-2"."*:align-bottom" {
            a href={"/admin/payments?member_search="(member.id)} ."btn"."btn-secondary"."btn-outline" {"View Payments"}
            @if !member.banned {
                button ."btn"."btn-secondary"."btn-outline" onclick="openModal()" hx-get={(nest.as_str())"/new_payment/"(member.id)} hx-target="#modal-content" {"Add Payment"}
                @if member.is_active == Some(true) {
                    button ."btn"."btn-secondary"."btn-outline" hx-post={(nest.as_str())"/send_discord_email/"(member.id)} hx-swap="none" {(icons::discord()) "Send Discord Invite"}
                }
                @if !member.cancelled {
                    button ."btn"."btn-secondary"."btn-outline" onclick="openModal()" hx-get={(nest.as_str())"/cancel/"(member.id)} hx-target="#modal-content" {(icons::warning()) "Cancel"}
                }
                button ."btn"."btn-secondary"."btn-outline" onclick="openModal()" hx-get={(nest.as_str())"/ban/"(member.id)} hx-target="#modal-content" {(icons::warning()) "Ban"}
            } @else {
                button ."btn"."btn-secondary"."btn-outline" onclick="openModal()" hx-get={(nest.as_str())"/unban/"(member.id)} hx-target="#modal-content" {(icons::warning()) "Unban"}
            }
        }
    })
}

pub async fn send_discord_email(
    State(state): State<crate::AppState>,
    Path(member_id): Path<i32>,
) -> Result<Markup, Response> {
    let email = sqlx::query_scalar!(
        r#"SELECT first_name||' '||last_name||' <'||email||'>' AS "id!" FROM members WHERE id=$1"#,
        member_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err_response(ErrorResponse::Toast)?;
    let first_name =
        sqlx::query_scalar!(r#"SELECT first_name FROM members WHERE id=$1"#, member_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err_response(ErrorResponse::Toast)?;

    let mailer = build_mailer(&state)
        .await
        .map_err_response(ErrorResponse::Toast)?;
    let invite_url = create_invite(
        Some(&format!("Manual send to {} from MDMA Web UI", email)),
        &state,
    )
    .await
    .map_err_response(ErrorResponse::Toast)?;
    let message = build_message(
        "discord",
        "Psychedelic Club Discord",
        &email,
        &EmailValues {
            first_name,
            invite_url,
            ..Default::default()
        },
        &state,
    )
    .await
    .map_err_response(ErrorResponse::Toast)?;

    mailer
        .send(message)
        .await
        .map_err_response(ErrorResponse::Toast)?;
    Ok(html! {
        (components::ToastAlert::Success(&format!("Sent Discord Invite to {}", email)))
    })
}
