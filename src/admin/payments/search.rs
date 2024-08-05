use axum::{
    extract::{NestedPath, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use maud::{html, Markup};
use reqwest::StatusCode;
use tokio::try_join;

use crate::{db::payments::PaymentsQuery, err_responses::{ErrorResponse, MapErrorResponse}, icons};

pub async fn search_form(nest: NestedPath, Query(params): Query<PaymentsQuery>) -> Markup {
    html! { #"payments_list" ."w-full"."max-w-4xl"."mx-auto" {
        #"payments_search" ."card"."bg-base-200"."w-full"."border"."border-secondary" {
            form hx-get={(nest.as_str())"/search"} hx-target="#payments_search_results" hx-push-url="true" ."card-body" {
                ."card-title" {"Search Payments"}
                label ."input"."input-bordered"."flex"."items-center"."gap-2" {
                    input type="text" name="member_search" placeholder="Search by Member" value=[&params.member_search] ."grow"."bg-inherit";
                    span ."text-secondary" {(icons::search())}
                }
                ."divider" {"Sort Results"}
                ."form-control" {
                    label ."label"."cursor-pointer" {
                        span ."label-text" {"Sort By"}
                        select name="sort_by" ."select"."select-bordered" {
                            option value="effective_on" selected[params.sort_by=="effective_on"] {"Effective Date"}
                            option value="amount_paid" selected[params.sort_by=="amount_paid"] {"Amount Paid"}
                        }
                    }
                }
                ."form-control" {
                    label ."label"."cursor-pointer" {
                        span ."label-text" {"Descending"}
                        input type="checkbox" name="sort_desc" value="true" checked[params.sort_desc] ."checkbox"."checkbox-primary";
                    }
                }
                ."card-actions"."justify-center" {
                    button ."btn"."btn-primary"."w-1/2"."block"."mx-auto"."!mb-0" {"SEARCH"}
                }
            }
        }
        ."divider" {}
        #"payments_search_results" hx-get={(nest.as_str())"/search"} hx-trigger="load" hx-vals=(serde_json::to_string(&params).unwrap()) {}
    } }
}

pub struct PaginationRequest {
    count: u64,
    offset: u64,
}

pub async fn search_results(
    headers: HeaderMap,
    nest: NestedPath,
    Query(params): Query<PaymentsQuery>,
    State(state): State<crate::AppState>,
) -> Result<Markup, Response> {
    if headers.contains_key("X-Rebuild-Page") {
        return Ok(search_form(nest, Query(params)).await);
    }

    let (payments, total) = try_join!(
        crate::db::payments::search(&params, &state),
        crate::db::payments::count(&params, &state)
    )
    .map_err_response(ErrorResponse::InternalServerError)?;

    let pagebtn = |request_opt: Option<PaginationRequest>, text: &str| -> Markup {
        html! {
            @if let Some(request) = request_opt {
                button ."btn"."btn-outline"."join-item"."w-1/4" hx-get={(nest.as_str())"/search"} hx-target="#payments_search_results"
                    hx-vals=(serde_json::to_string(&PaymentsQuery {count: request.count, offset: request.offset, ..params.clone()}).unwrap()) {(text)}
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
        ."overflow-x-auto" { table ."table"."table-zebra"."table-auto"."[&_td]:whitespace-nowrap" {
            thead { tr {
                th {"Member Name"}
                th {"Member Email"}
                th {"Effective On"}
                th {"Amount Paid"}
                th {"Payment Method"}
                th {"Open"}
            }}
            @for payment in &payments {
                tr {
                    td { a href={"/admin/members?search="(payment.email)} target="_blank" ."btn"."btn-link" {(payment.last_name)", "(payment.first_name)} }
                    td { a href={"mailto:"(payment.email)} ."btn"."btn-link" {(payment.email)} }
                    td {(payment.effective_on)}
                    td {"$"(payment.amount_paid.round_dp(2))}
                    td {(payment.payment_method.as_deref().unwrap_or_default())}
                    td {
                        @match payment.platform.as_deref() {
                            Some("webconnex") => { a href={"/.webconnex/redirect/transaction/"(payment.transaction_id.unwrap_or_default())} target="_blank" ."btn"."btn-circle"."btn-outline" {(icons::open_external())} },
                            _ => {}
                        }
                    }
                }
            }
        }}
        ."divider" {}
        #"payments_pagination" ."join"."join-vertical"."md:join-horizontal"."justify-center"."w-full"."items-center" {
            (pagebtn(prev, "Previous"))
            ."btn"."btn-outline"."join-item"."w-1/4"."!text-neutral-content" disabled {
                (params.offset + 1)" - "(params.offset + (payments.len() as u64))" of "(total)
            }
            (pagebtn(next, "Next"))
        }
    })
}
