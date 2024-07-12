use axum::extract::State;
use maud::{html, Markup};
use time::Date;

use crate::icons;

pub struct GenerationStats {
    id: i32,
    title: String,
    start_date: Date,
    total_members: i64,
    active_members: i64,
    active_emails: Vec<String>,
}

impl GenerationStats {
    pub fn percent_active(&self) -> f64 {
        self.active_members as f64 / self.total_members as f64
    }
}

pub async fn generations_list(State(state): State<crate::AppState>) -> Markup {
    let generations = sqlx::query_as!(
        GenerationStats,
        r#"SELECT 
            id AS "id!",
            total_members AS "total_members!",
            active_members AS "active_members!",
            COALESCE(active_emails, '{}') AS "active_emails!",
            title,
            start_date
        FROM 
            (SELECT generation_id AS id,
                    COUNT(*) AS total_members,
                    COUNT(*) FILTER (WHERE consecutive_until_cached >= NOW()) AS active_members,
                    ARRAY_AGG(CONCAT(first_name, ' ', last_name, ' <', email, '>')) FILTER (WHERE consecutive_until_cached >= NOW()) AS active_emails
            FROM member_generations
                INNER JOIN members ON members.id = member_id
            GROUP BY generation_id) temp1
        INNER JOIN generations USING (id)
        ORDER BY start_date ASC"#
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap();

    let total_members: i64 = generations.iter().map(|gen| gen.total_members).sum();
    let active_members: i64 = generations.iter().map(|gen| gen.active_members).sum();
    let all_emails = generations
        .iter()
        .map(|gen| gen.active_emails.join(";"))
        .collect::<Vec<String>>()
        .join(";");

    html! {
        table #"generations-list"."table"."mx-auto" {
            thead {
                th {}
                th {"Name"}
                th {"Start Date"}
                th {"Total Members"}
                th {"Active Members"}
                th {"Percent Active"}
                th {"Actions"}
            }
            tbody {
                @for gen in generations {
                    tr {
                        td {(gen.id)}
                        td {(gen.title)}
                        td {(gen.start_date)}
                        td {(gen.total_members)}
                        td {(gen.active_members)}
                        td {(format!("{:.1}", gen.percent_active() * 100.0)) "%"}
                        td ."[&>*]:mx-1" {
                            ."tooltip" data-tip="Email Active Members" {
                                a ."btn"."btn-circle"."btn-outline"."btn-secondary"
                                    href={"mailto:?bcc=" (gen.active_emails.join(","))}
                                    { (icons::envelope()) }

                            }
                            ."tooltip" data-tip="Copy Active Member Emails" {
                                a ."btn"."btn-circle"."btn-outline"."btn-secondary"
                                    href="#"
                                    onclick={"navigator.clipboard.writeText('" (gen.active_emails.join(",")) "')"}
                                    { (icons::copy()) }
                            }
                        }
                    }
                }
                tr ."bg-base-300"."border-t-2" {
                    td ."rounded-bl-lg" {}
                    td {"Total"}
                    td {}
                    td {(total_members)}
                    td {(active_members)}
                    td {(format!("{:.1}", active_members as f64 / total_members as f64 * 100.0)) "%"}
                    td ."rounded-br-lg"."[&>*]:mx-1" {
                        ."tooltip" data-tip="Email Active Members" {
                            a ."btn"."btn-circle"."btn-outline"."btn-primary"
                                href={"mailto:?bcc=" (all_emails)}
                                { (icons::envelope()) }
                        }
                        ."tooltip" data-tip="Copy Active Member Emails" {
                            a ."btn"."btn-circle"."btn-outline"."btn-primary"
                                href="#"
                                onclick={"navigator.clipboard.writeText('" (all_emails) "')"}
                                { (icons::copy()) }
                        }
                    }
                }
            }
        }
    }
}
