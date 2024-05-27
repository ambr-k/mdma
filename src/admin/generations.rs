use askama::Template;
use axum::extract::State;
use time::Date;

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

#[derive(Template)]
#[template(path = "admin/generations.html")]
pub struct UsersListTemplate {
    generations: Vec<GenerationStats>,
}

pub async fn generations_list(State(state): State<crate::AppState>) -> UsersListTemplate {
    let generations = sqlx::query_as!(
        GenerationStats,
        r#"SELECT 
            id AS "id!",
            total_members AS "total_members!",
            active_members AS "active_members!",
            active_emails AS "active_emails!",
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

    UsersListTemplate { generations }
}
