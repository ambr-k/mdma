use axum::{
    extract::{NestedPath, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use maud::{html, Markup};
use reqwest::StatusCode;
use tokio::try_join;

use crate::{db::members::MembersQuery, icons};

struct SelectIdOption {
    id: i32,
    description: String,
}

pub struct PaginationRequest {
    count: u64,
    offset: u64,
}

pub async fn members_list(
    nest: NestedPath,
    Query(params): Query<MembersQuery>,
    State(state): State<crate::AppState>,
) -> Markup {
    let generation_options = sqlx::query_as!(
        SelectIdOption,
        r#"SELECT id, CONCAT(title, ' (', start_date, ')') AS "description!" FROM generations"#
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    html! { #"members-list" ."w-full"."max-w-xl"."mx-auto" {
        #"members-search" ."collapse"."collapse-arrow"."bg-base-200"."my-2"."border"."border-secondary" {
            input type="radio" name="members-list-accordion" checked;
            ."collapse-title"."text-xl"."font-medium" {"Search Members"}
            ."collapse-content" {
                form ."[&>*]:my-3" hx-get={(nest.as_str())"/search"} hx-target="#members-search-results" hx-push-url="true" {
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
                            span ."label-text" {"Active Status"}
                            select name="member_status" ."select"."select-bordered" {
                                option value="" selected[params.member_status.is_none()] {"(Ignore)"}
                                option value="true" selected[params.member_status==Some(true)] {"Active"}
                                option value="false" selected[params.member_status==Some(false)] {"Inactive"}
                            }
                        }
                    }
                    ."form-control" {
                        label ."label"."cursor-pointer" {
                            span ."label-text" {"Discord Status"}
                            select name="discord_status" ."select"."select-bordered" {
                                option value="" selected[params.discord_status.is_none()] {"(Ignore)"}
                                option value="true" selected[params.discord_status==Some(true)] {"Registered"}
                                option value="false" selected[params.discord_status==Some(false)] {"Unregistered"}
                            }
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
        #"members-search-results" hx-get={(nest.as_str())"/search"} hx-trigger="load" hx-vals=(serde_json::to_string(&params).unwrap())  {}
    }}
}

pub async fn search_results(
    headers: HeaderMap,
    nest: NestedPath,
    Query(params): Query<MembersQuery>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    if headers.contains_key("X-Rebuild-Page") {
        return Ok(members_list(nest, Query(params), State(state)).await);
    }

    let (members, total) = try_join!(
        crate::db::members::search(&params, &state),
        crate::db::members::count(&params, &state)
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?;

    let pagebtn = |request_opt: Option<PaginationRequest>, text: &str| -> Markup {
        html! {
            @if let Some(request) = request_opt {
                button ."btn"."btn-outline"."join-item"."w-1/4" hx-get={(nest.as_str())"/search"} hx-target="#members-search-results"
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

    Ok(html! {
        @for member in &members {
            ."collapse"."collapse-arrow"."bg-base-200"."my-2" {
                input #{"user_details_trigger_"(member.id)} type="radio" name="members-list-accordion"
                    hx-get={(nest.as_str())"/details/"(member.id)} hx-target="next .collapse-content" hx-indicator="closest .collapse";
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
    })
}
