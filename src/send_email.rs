use lettre::{
    message::{Mailbox, SinglePart},
    transport::smtp::authentication::{Credentials, Mechanism},
    AsyncSmtpTransport, Message, Tokio1Executor,
};
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use serde::{Deserialize, Serialize};
use tinytemplate::TinyTemplate;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct EmailValues {
    pub first_name: String,
    pub last_name: String,
    pub invite_url: String,
    pub email: String,
    pub timestamp: String,
    pub amount_paid: String,
    pub donor_id: String,
    pub donor_url: String,
    pub donation_id: String,
    pub donation_url: String,
    pub plan_id: String,
    pub plan_url: String,
    pub payment_id: String,
    pub payment_url: String,
    pub referral_source: String,
}

pub async fn get_email_template(id: &str, db_pool: &sqlx::PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!("SELECT template FROM email_templates WHERE id = $1", id)
        .fetch_one(db_pool)
        .await
}

pub async fn insert_email_template(
    id: &str,
    template: &str,
    db_pool: &sqlx::PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO email_templates (id, template) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET template = excluded.template",
        id,
        template
    ).execute(db_pool).await.map(|_| ())
}

pub async fn insert_email_address(
    id: &str,
    template: &str,
    db_pool: &sqlx::PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO email_addresses (id, value) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET value = excluded.value",
        id,
        template
    ).execute(db_pool).await.map(|_| ())
}

pub async fn get_email_address(id: &str, db_pool: &sqlx::PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!("SELECT value FROM email_addresses WHERE id = $1", id)
        .fetch_one(db_pool)
        .await
}

pub fn sanitize_email(contents: &str) -> String {
    ammonia::Builder::new()
        .add_generic_attributes(&["style"])
        .clean(&contents)
        .to_string()
}

fn populate_email_template(
    template: &str,
    values: &EmailValues,
) -> Result<String, tinytemplate::error::Error> {
    let mut templ = TinyTemplate::new();
    templ.add_template("email_template", &template)?;
    templ
        .render("email_template", values)
        .map(|populated| sanitize_email(&populated))
}

async fn get_access_token(state: &crate::AppState) -> Result<AccessToken, String> {
    state
        .google_oauth
        .exchange_refresh_token(&RefreshToken::new(
            state.secret_store.get("GMAIL_OAUTH_REFRESH_TOKEN").unwrap(),
        ))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map(|resp| resp.access_token().to_owned())
        .map_err(|err| err.to_string())
}

pub async fn build_mailer(
    state: &crate::AppState,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    Ok(
        AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
            .map_err(|err| err.to_string())?
            .authentication(vec![Mechanism::Xoauth2])
            .credentials(Credentials::new(
                state.secret_store.get("GMAIL_USERNAME").unwrap(),
                get_access_token(state).await?.secret().clone(),
            ))
            .build(),
    )
}

pub async fn build_message(
    email_key: &str,
    subject: &str,
    to_address: &str,
    values: &EmailValues,
    state: &crate::AppState,
) -> Result<Message, String> {
    let from_mbox = get_email_address("from", &state.db_pool)
        .await
        .map_err(|err| err.to_string())?
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;
    let replyto_mbox = get_email_address("replyto", &state.db_pool)
        .await
        .map_err(|err| err.to_string())?
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;
    let to_mbox = to_address
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;

    let email_template = get_email_template(email_key, &state.db_pool)
        .await
        .map_err(|err| err.to_string())?;
    let email_body =
        populate_email_template(&email_template, values).map_err(|err| err.to_string())?;

    Message::builder()
        .from(from_mbox)
        .reply_to(replyto_mbox)
        .to(to_mbox)
        .subject(subject)
        .singlepart(SinglePart::html(email_body))
        .map_err(|err| err.to_string())
}
