use std::str::FromStr;

use rust_decimal::Decimal;
use sea_query::{
    extension::postgres::PgExpr, Asterisk, Expr, Iden, IntoColumnRef, Order, PostgresQueryBuilder,
    Query, SimpleExpr,
};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Deserializer, Serialize};
use serde_inline_default::serde_inline_default;
use sqlx::prelude::FromRow;
use time::Date;

fn none_or_empty(val: &Option<String>) -> bool {
    match val.as_deref() {
        None => true,
        Some("") => true,
        Some(_) => false,
    }
}

fn deserialize_option_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(bool::deserialize(deserializer).ok())
}

#[serde_inline_default]
#[derive(Deserialize, Serialize, Clone)]
pub struct MembersQuery {
    #[serde(skip_serializing_if = "none_or_empty")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "none_or_empty")]
    pub discord: Option<String>,

    #[serde(
        deserialize_with = "deserialize_option_bool",
        skip_serializing_if = "Option::is_none"
    )]
    #[serde_inline_default(None)]
    pub member_status: Option<bool>,
    #[serde(
        deserialize_with = "deserialize_option_bool",
        skip_serializing_if = "Option::is_none"
    )]
    #[serde_inline_default(None)]
    pub discord_status: Option<bool>,

    #[serde_inline_default(12)]
    pub count: u64,

    #[serde_inline_default(0)]
    pub offset: u64,

    #[serde_inline_default(-1)]
    pub generation_id: i32,

    #[serde_inline_default(String::from(""))]
    pub sort_by: String,

    #[serde_inline_default(false)]
    pub sort_desc: bool,
}

#[allow(dead_code)]
#[derive(FromRow)]
pub struct MemberDetailsRow {
    pub id: i32,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub notes: String,
    pub created_on: Date,
    pub discord: Option<Decimal>,
    pub cancelled: bool,
    pub banned: bool,
    pub first_payment: Option<Date>,
    pub consecutive_since: Option<Date>,
    pub consecutive_until: Option<Date>,
    pub generation_name: Option<String>,
    pub generation_id: Option<i32>,
    pub is_active: Option<bool>,
}

#[allow(dead_code)]
#[derive(FromRow)]
pub struct MemberRow {
    pub id: i32,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub notes: String,
    pub created_on: Date,
    pub discord: Option<Decimal>,
    pub cancelled: bool,
    pub banned: bool,
}

trait MembersQueryFilter {
    fn from_member_details(&mut self) -> &mut Self;
    fn members_query_filter(&mut self, params: &MembersQuery) -> &mut Self;
    async fn members_discord_filter(
        &mut self,
        discord: &Option<String>,
        http: &serenity::http::Http,
        guildid: &serenity::model::id::GuildId,
    ) -> &mut Self;
}

impl MembersQueryFilter for sea_query::SelectStatement {
    fn from_member_details(&mut self) -> &mut Self {
        self.from(Members::Table).left_join(
            MemberDetails::Table,
            Expr::col((Members::Table, Members::Id))
                .equals((MemberDetails::Table, MemberDetails::Id)),
        )
    }

    fn members_query_filter(&mut self, params: &MembersQuery) -> &mut Self {
        self.conditions(
            !params
                .search
                .as_ref()
                .unwrap_or(&String::from(""))
                .is_empty(),
            |q| {
                q.and_where(
                    Expr::col(Members::FirstName)
                        .concat(SimpleExpr::Constant(" ".into()))
                        .concat(Expr::col(Members::LastName))
                        .ilike(format!("%{}%", params.search.as_ref().unwrap()))
                        .or(Expr::col(Members::Email)
                            .ilike(format!("%{}%", params.search.as_ref().unwrap()))),
                );
            },
            |_| {},
        )
        .conditions(
            params.member_status.is_some(),
            |q| {
                q.and_where(Expr::col(MemberDetails::IsActive).eq(params.member_status.unwrap()));
            },
            |_| {},
        )
        .conditions(
            params.discord_status.is_some(),
            |q| {
                q.and_where(if params.discord_status.unwrap() {
                    Expr::col(Members::Discord).is_not_null()
                } else {
                    Expr::col(Members::Discord).is_null()
                });
            },
            |_| {},
        )
        .conditions(
            params.generation_id >= 0,
            |q| {
                q.and_where(
                    Expr::col(MemberDetails::GenerationId).eq(Expr::value(params.generation_id)),
                );
            },
            |_| {},
        )
    }

    async fn members_discord_filter(
        &mut self,
        discord: &Option<String>,
        http: &serenity::http::Http,
        guildid: &serenity::model::id::GuildId,
    ) -> &mut Self {
        let userids = match discord.as_deref() {
            Some("") => None,
            Some(val) => http
                .search_guild_members(*guildid, val, Some(1000))
                .await
                .ok(),
            None => None,
        }
        .map(|v| {
            v.iter()
                .map(|m| Decimal::from(m.user.id.get()))
                .collect::<Vec<_>>()
        });

        self.conditions(
            userids.is_some(),
            |q| {
                let username_matches = Expr::col(Members::Discord).is_in(userids.unwrap());
                q.and_where(match Decimal::from_str(discord.as_deref().unwrap()) {
                    Ok(userid) => username_matches.or(Expr::col(Members::Discord).eq(userid)),
                    Err(_) => username_matches,
                });
            },
            |_| {},
        )
    }
}

pub async fn search(
    params: &MembersQuery,
    state: &crate::AppState,
) -> Result<Vec<MemberRow>, sqlx::Error> {
    let sort_order = if params.sort_desc {
        Order::Desc
    } else {
        Order::Asc
    };

    let (query, values) = Query::select()
        .column(Asterisk)
        .from_member_details()
        .members_query_filter(params)
        .members_discord_filter(&params.discord, &state.discord_http, &state.discord_guild)
        .await
        .limit(params.count)
        .offset(params.offset)
        .order_by_columns(match params.sort_by.as_str() {
            "firstname" => vec![
                (Members::FirstName.into_column_ref(), sort_order.clone()),
                (Members::LastName.into_column_ref(), sort_order.clone()),
            ],
            "lastname" => vec![
                (Members::LastName.into_column_ref(), sort_order.clone()),
                (Members::FirstName.into_column_ref(), sort_order.clone()),
            ],
            "consecutivesince" => vec![(
                MemberDetails::ConsecutiveSince.into_column_ref(),
                sort_order.clone(),
            )],
            _ => vec![(
                (Members::Table, Members::Id).into_column_ref(),
                sort_order.clone(),
            )],
        })
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, MemberRow, _>(&query, values)
        .fetch_all(&state.db_pool)
        .await
}

pub async fn count(params: &MembersQuery, state: &crate::AppState) -> Result<u64, sqlx::Error> {
    let (query, values) = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from_member_details()
        .members_query_filter(params)
        .members_discord_filter(&params.discord, &state.discord_http, &state.discord_guild)
        .await
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_scalar_with::<_, i64, _>(&query, values)
        .fetch_one(&state.db_pool)
        .await
        .and_then(|r| Ok(r.try_into().unwrap()))
}

#[allow(dead_code)]
#[derive(Iden)]
pub enum Members {
    Table,
    Id,
    Email,
    FirstName,
    LastName,
    ReasonRemoved,
    CreatedOn,
    Discord,
}

#[derive(Iden)]
enum MemberDetails {
    Table,
    Id,
    GenerationId,
    GenerationName,
    ConsecutiveSince,
    ConsecutiveUntil,
    IsActive,
    FirstPayment,
}
