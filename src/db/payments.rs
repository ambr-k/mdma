use std::str::FromStr;

use rust_decimal::Decimal;
use sea_query::{
    extension::postgres::PgExpr, Asterisk, Expr, Iden, PostgresQueryBuilder, Query, SimpleExpr,
};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use sqlx::FromRow;
use time::Date;

use super::members::Members;

#[serde_inline_default]
#[derive(Deserialize, Serialize, Clone)]
pub struct PaymentsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_search: Option<String>,

    #[serde_inline_default(12)]
    pub count: u64,
    #[serde_inline_default(0)]
    pub offset: u64,
}

#[derive(FromRow)]
pub struct PaymentRow {
    pub id: i32,
    pub member_id: i32,
    pub effective_on: Date,
    pub created_on: Date,
    pub duration_months: i32,
    pub amount_paid: Decimal,
    pub payment_method: Option<String>,
    pub platform: Option<String>,
    pub transaction_id: Option<i32>,
    pub notes: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

trait PaymentsQueryFilter {
    fn payments_query_filter(&mut self, params: &PaymentsQuery) -> &mut Self;
}

impl PaymentsQueryFilter for sea_query::SelectStatement {
    fn payments_query_filter(&mut self, params: &PaymentsQuery) -> &mut Self {
        self.conditions(
            params.member_search.is_some(),
            |q| {
                let member_id = Decimal::from_str(params.member_search.as_deref().unwrap()).ok();
                q.conditions(
                    member_id.is_some(),
                    |q| {
                        q.and_where(Expr::col(Payments::MemberId).eq(member_id.unwrap()));
                    },
                    |q| {
                        q.and_where(
                            Expr::col(Members::FirstName)
                                .concat(SimpleExpr::Constant(" ".into()))
                                .concat(Expr::col(Members::LastName))
                                .ilike(format!("%{}%", params.member_search.as_ref().unwrap()))
                                .or(Expr::col(Members::Email).ilike(format!(
                                    "%{}%",
                                    params.member_search.as_ref().unwrap()
                                ))),
                        );
                    },
                );
            },
            |_| {},
        )
    }
}

pub async fn search(
    params: &PaymentsQuery,
    state: &crate::AppState,
) -> Result<Vec<PaymentRow>, sqlx::Error> {
    let (query, values) = Query::select()
        .column((Payments::Table, Asterisk))
        .column((Members::Table, Asterisk))
        .from(Payments::Table)
        .inner_join(
            Members::Table,
            Expr::col(Payments::MemberId).equals((Members::Table, Members::Id)),
        )
        .payments_query_filter(params)
        .limit(params.count)
        .offset(params.offset)
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, PaymentRow, _>(&query, values)
        .fetch_all(&state.db_pool)
        .await
}

pub async fn count(params: &PaymentsQuery, state: &crate::AppState) -> Result<u64, sqlx::Error> {
    let (query, values) = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from(Payments::Table)
        .inner_join(
            Members::Table,
            Expr::col(Payments::MemberId).equals((Members::Table, Members::Id)),
        )
        .payments_query_filter(params)
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_scalar_with::<_, i64, _>(&query, values)
        .fetch_one(&state.db_pool)
        .await
        .and_then(|r| Ok(r.try_into().unwrap()))
}

#[derive(Iden)]
#[allow(dead_code)]
enum Payments {
    Table,
    Id,
    MemberId,
    EffectiveOn,
    CreatedOn,
    DurationMonths,
    AmountPaid,
    PaymentMethod,
    Platform,
    TransactionId,
    Notes,
}
