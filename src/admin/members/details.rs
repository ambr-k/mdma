use axum::{
    extract::{NestedPath, Path, State},
    response::{IntoResponse, Response},
};
use maud::{html, Markup};
use reqwest::StatusCode;
use rust_decimal::prelude::ToPrimitive;
use serde::Deserialize;

use crate::{db::members::MemberRow, icons};

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
        MemberRow,
        r#" SELECT members.*, generations.title AS "generation_name?"
            FROM members
                LEFT JOIN member_generations ON members.id = member_id
                LEFT JOIN generations ON generations.id = generation_id
            WHERE members.id=$1"#,
        member_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, err.to_string()).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    })?;

    // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

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
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .json::<WebconnexCustomerSearchResponse>()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .data
        .unwrap_or_default();

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
        a ."btn"."btn-link" href={"mailto:"(member.email)} {(member.email)}
        @if let Some(val) = member.consecutive_since_cached {
            p {"Active since "(val)" ("(member.generation_name.as_deref().unwrap_or("Generation N/A"))")"}
        }
        @if let Some(val) = member.consecutive_until_cached {
            p {"Active until "(val)}
        }
        @if member.consecutive_until_cached.is_none() { p {"No recorded payments"} }
        ."divider" {"Third-Party Accounts"}
        @for wc_account in webconnex {
            a ."btn"."btn-outline"."btn-primary" href={"https://manage.webconnex.com/contacts/"(wc_account.id)} target="_blank" {"GivingFuel ID "(wc_account.id)}
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
        ."divider" {"Actions"}
        button ."btn"."btn-secondary"."btn-outline" onclick="openModal()" hx-get={(nest.as_str())"/new_payment/"(member.id)} hx-target="#modal-content" {"Add Payment"}
        a href={"/admin/payments?member_search="(member.id)} ."btn"."btn-secondary"."btn-outline"."mx-2" {"View Payments"}
    })
}
