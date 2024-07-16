use axum::{
    extract::{NestedPath, Query, State},
    routing::get,
    Router,
};
use maud::{html, Markup};
use rust_decimal::{
    prelude::{ToPrimitive, Zero},
    Decimal,
};
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serenity::all::RoleId;

async fn search_form(nest: NestedPath, State(state): State<crate::AppState>) -> Markup {
    let roles = state
        .discord_http
        .get_guild_roles(state.discord_guild)
        .await
        .unwrap();

    html! { #"discord_audit" ."w-full"."max-w-4xl"."mx-auto" {
        form hx-get={(nest.as_str())"/search"} hx-target="#discord_audit_search" hx-indicator="#discord_audit_search_loading" {
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Discord Role"}
                    select name="discord_role" ."select"."select-bordered" {
                        option value="0" selected {"@everyone"}
                        @for role in roles {
                            option value=(role.id.get()) {(role.name)}
                        }
                    }
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Include inactive members' accounts"}
                    input type="checkbox" name="include_inactive" value="true" checked ."checkbox"."checkbox-primary";
                }
            }
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Include accounts not registered in MDMA"}
                    input type="checkbox" name="include_unregistered" value="true" checked ."checkbox"."checkbox-primary";
                }
            }
            button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0" {"SEARCH"}
            progress #"discord_audit_search_loading" ."progress"."htmx-indicator" {}
        }
        #"discord_audit_search" {}
        #"discord_audit_response" {}
    }}
}

#[serde_inline_default]
#[derive(Deserialize)]
struct SearchQuery {
    #[serde_inline_default(Decimal::zero())]
    discord_role: Decimal,
    #[serde_inline_default(false)]
    include_inactive: bool,
    #[serde_inline_default(false)]
    include_unregistered: bool,
}

#[derive(Deserialize, Debug)]
struct SqlSearchResult {
    discord_id: Decimal,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
}

async fn search_results(
    nest: NestedPath,
    Query(params): Query<SearchQuery>,
    State(state): State<crate::AppState>,
) -> Markup {
    let role_id = if params.discord_role == Decimal::zero() {
        None
    } else {
        Some(RoleId::new(params.discord_role.to_u64().unwrap()))
    };

    let all_users = state
        .discord_http
        .get_guild_members(state.discord_guild, Some(1000), None)
        .await
        .unwrap();
    if all_users.len() >= 1000 {
        return html! { p { "Holy shit the Discord server has more than 1000 members... that's crazy I guess I need to update MDMA to support a huge server wow" } };
    }

    let role_users = all_users
        .iter()
        .filter(|d| role_id.is_none() || d.roles.contains(role_id.as_ref().unwrap()))
        .collect::<Vec<_>>();
    let user_ids = role_users
        .iter()
        .map(|a| Decimal::from(a.user.id.get()))
        .collect::<Vec<_>>();

    let db_records = sqlx::query_as!(
        SqlSearchResult,
        r#"
            SELECT discord_id as "discord_id!", members.first_name as "first_name?", members.last_name as "last_name?", members.email as "email?"
            FROM unnest($1::numeric[]) discord_id
                LEFT OUTER JOIN members ON discord_id=members.discord
            WHERE
                ($2 AND consecutive_until_cached < NOW())
                OR ($3 AND members.id IS NULL)
        "#,
        &user_ids,
        params.include_inactive,
        params.include_unregistered,
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    let user_map = role_users
        .iter()
        .map(|m| (Decimal::from(m.user.id.get()), m))
        .collect::<std::collections::HashMap<_, _>>();
    let results = db_records
        .iter()
        .map(|r| (r, user_map.get(&r.discord_id).unwrap()))
        .collect::<Vec<_>>();

    html! {
        ."divider" {(results.len())" Results"}
        form hx-post={(nest.as_str())"/execute"} hx-target="#discord_audit_response" {
            ."max-h-96"."overflow-auto" {
                table ."table"."table-zebra"."max-h-96"."overflow-y-auto" {
                    thead { tr { th {} th {"Discord"} th {"First Name"} th {"Last Name"} th {"Email"} } }
                    tbody { @for (mdma_member, discord_member) in results {
                            tr {
                                td { label { input type="checkbox" name=(discord_member.user.id) value="true" checked ."checkbox"."checkbox-primary"; } }
                                td { a href={"/admin/members?discord="(discord_member.user.id)} ."btn"."btn-link" {(discord_member.display_name())} }
                                td { (mdma_member.first_name.as_deref().unwrap_or_default()) }
                                td { (mdma_member.last_name.as_deref().unwrap_or_default()) }
                                td { @if let Some(email) = &mdma_member.email { a href={"mailto:"(email)} ."btn"."btn-link" {(email)} } }
                            }
                    }}
                }
            }
            ."divider" {"Actions"}
            ."form-control" {
                label ."label"."cursor-pointer" {
                    span ."label-text" {"Action"}
                    select name="action" ."select"."select-bordered" {
                        option value="send_dm" selected {"Send DM"}
                        option value="remove_role" {"Remove Role"}
                    }
                }
            }
            input type="hidden" name="role" value=(params.discord_role);
            textarea name="details" placeholder="DM Contents" ."textarea"."textarea-bordered"."w-full" {}
            label ."form-control"."w-full" {
                ."label" { span ."label-text" {"Enter your email to prove you know what you're doing..."} }
                input type="text" name="email-verify" placeholder="Email" ."input"."input-bordered";
            }
            button ."btn"."btn-secondary"."w-1/3"."mx-auto"."mt-4" {"SUBMIT"}
        }
    }
}

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", get(search_form))
        .route("/search", get(search_results))
        .with_state(state.clone())
}
