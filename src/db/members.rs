use rust_decimal::Decimal;
use sea_query::{
    extension::postgres::PgExpr, Alias, Asterisk, Expr, Iden, Order, PostgresQueryBuilder, Query,
    SimpleExpr,
};
use sea_query_binder::SqlxBinder;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use sqlx::{prelude::FromRow, PgPool};
use time::Date;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct MembersQuery {
    pub search: Option<String>,

    #[serde_inline_default(false)]
    pub active_only: bool,

    #[serde_inline_default(12)]
    pub count: u64,

    #[serde_inline_default(0)]
    pub offset: u64,

    #[serde_inline_default(-1)]
    pub generation_id: i32,

    #[serde_inline_default(0)]
    pub sort_by: u64,

    #[serde_inline_default(false)]
    pub sort_desc: bool,
}

#[allow(dead_code)]
#[derive(FromRow)]
pub struct MemberRow {
    pub id: i32,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub reason_removed: Option<String>,
    pub created_on: Date,
    pub consecutive_since_cached: Option<Date>,
    pub consecutive_until_cached: Option<Date>,
    pub generation_name: Option<String>,
    pub discord: Option<Decimal>,
}

trait MembersQueryFilter {
    fn members_query_filter(&mut self, params: &MembersQuery) -> &mut Self;
}

impl MembersQueryFilter for sea_query::SelectStatement {
    fn members_query_filter(&mut self, params: &MembersQuery) -> &mut Self {
        self.conditions(
            params.search.is_some(),
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
            params.active_only,
            |q| {
                q.and_where(
                    Expr::col(Members::ConsecutiveUntilCached)
                        .gt(Expr::current_date())
                        .and(Expr::col(Members::ReasonRemoved).is_null()),
                );
            },
            |_| {},
        )
        .conditions(
            params.generation_id >= 0,
            |q| {
                q.and_where(
                    Expr::col(MemberGenerations::GenerationId)
                        .eq(Expr::value(params.generation_id)),
                );
            },
            |_| {},
        )
    }
}

pub async fn search(
    params: &MembersQuery,
    db_pool: &PgPool,
) -> Result<Vec<MemberRow>, sqlx::Error> {
    let sort_order = if params.sort_desc {
        Order::Desc
    } else {
        Order::Asc
    };

    let (query, values) = Query::select()
        .column((Members::Table, Asterisk))
        .expr_as(Expr::col(Generations::Title), Alias::new("generation_name"))
        .from(Members::Table)
        .left_join(
            MemberGenerations::Table,
            Expr::col((Members::Table, Members::Id)).equals(MemberGenerations::MemberId),
        )
        .left_join(
            Generations::Table,
            Expr::col((Generations::Table, Generations::Id))
                .equals(MemberGenerations::GenerationId),
        )
        .members_query_filter(params)
        .limit(params.count)
        .offset(params.offset)
        .order_by_columns(match params.sort_by {
            1 => vec![
                ((Members::Table, Members::FirstName), sort_order.clone()),
                ((Members::Table, Members::LastName), sort_order.clone()),
            ],
            2 => vec![
                ((Members::Table, Members::LastName), sort_order.clone()),
                ((Members::Table, Members::FirstName), sort_order.clone()),
            ],
            3 => vec![(
                (Members::Table, Members::ConsecutiveSinceCached),
                sort_order.clone(),
            )],
            _ => vec![((Members::Table, Members::Id), sort_order.clone())],
        })
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, MemberRow, _>(&query, values)
        .fetch_all(db_pool)
        .await
}

pub async fn count(params: &MembersQuery, db_pool: &PgPool) -> Result<u64, sqlx::Error> {
    let (query, values) = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from(Members::Table)
        .left_join(
            MemberGenerations::Table,
            Expr::col((Members::Table, Members::Id)).equals(MemberGenerations::MemberId),
        )
        .members_query_filter(params)
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_scalar_with::<_, i64, _>(&query, values)
        .fetch_one(db_pool)
        .await
        .and_then(|r| Ok(r.try_into().unwrap()))
}

#[allow(dead_code)]
#[derive(Iden)]
enum Members {
    Table,
    Id,
    Email,
    FirstName,
    LastName,
    ReasonRemoved,
    CreatedOn,
    ConsecutiveSinceCached,
    ConsecutiveUntilCached,
    GenerationName,
    Discord,
}

#[derive(Iden)]
enum Generations {
    Table,
    Id,
    Title,
}

#[derive(Iden)]
enum MemberGenerations {
    Table,
    MemberId,
    GenerationId,
}
