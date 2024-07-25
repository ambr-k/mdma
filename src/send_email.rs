use lettre::{
    message::{Mailbox, SinglePart},
    transport::smtp::authentication::{Credentials, Mechanism},
    AsyncSmtpTransport, Message, Tokio1Executor,
};
use oauth2::{AccessToken, RefreshToken, TokenResponse};
use serde::{Deserialize, Serialize};
use shuttle_persist::PersistInstance;
use tinytemplate::TinyTemplate;

#[derive(Deserialize, Serialize)]
pub struct EmailValues {
    pub first_name: String,
    pub invite_url: String,
}

pub fn sanitize_email(contents: &str) -> String {
    ammonia::Builder::new()
        .add_generic_attributes(&["style"])
        .clean(&contents)
        .to_string()
}

pub fn populate_email(
    email_key: &str,
    values: &EmailValues,
    persist: &PersistInstance,
) -> Result<String, tinytemplate::error::Error> {
    let raw_contents = persist
        .load::<String>(&format!("email.{email_key}"))
        .unwrap_or_default();
    let mut templ = TinyTemplate::new();
    templ.add_template("discord_email", &raw_contents)?;
    templ
        .render("discord_email", values)
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

pub fn build_message(
    email_key: &str,
    subject: &str,
    to_address: &str,
    values: &EmailValues,
    state: &crate::AppState,
) -> Result<Message, String> {
    let from_mbox = state
        .persist
        .load::<String>("from_address")
        .map_err(|err| err.to_string())?
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;
    let replyto_mbox = state
        .persist
        .load::<String>("replyto_address")
        .map_err(|err| err.to_string())?
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;
    let to_mbox = to_address
        .parse::<Mailbox>()
        .map_err(|err| err.to_string())?;

    Message::builder()
        .from(from_mbox)
        .reply_to(replyto_mbox)
        .to(to_mbox)
        .subject(subject)
        .singlepart(SinglePart::html(
            populate_email(email_key, values, &state.persist).map_err(|err| err.to_string())?,
        ))
        .map_err(|err| err.to_string())
}
