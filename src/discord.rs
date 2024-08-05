use std::str::FromStr;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use reqwest::header;
use rust_decimal::Decimal;
use serenity::{builder::*, model::prelude::*};
use time::macros::date;

use crate::db::members::MemberDetailsRow;

async fn whois(
    user_id: UserId,
    mdma_url: &str,
    db_pool: &sqlx::PgPool,
) -> Result<CreateInteractionResponse, StatusCode> {
    let result = sqlx::query_as!(
        MemberDetailsRow,
        "SELECT * FROM members NATURAL JOIN member_details WHERE discord=$1",
        Decimal::from(user_id.get())
    )
    .fetch_all(db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.is_empty() {
        Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .flags(InteractionResponseFlags::SUPPRESS_NOTIFICATIONS)
                .ephemeral(true)
                .content(format!("<@{}> is not registered in MDMA", user_id.get())),
        ))
    } else {
        Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .flags(InteractionResponseFlags::SUPPRESS_NOTIFICATIONS)
                .ephemeral(true)
                .content(format!("<@{}> is registered in MDMA", user_id.get()))
                .button(
                    CreateButton::new_link(format!(
                        "https://{mdma_url}/admin/members/search?discord={user_id}"
                    ))
                    .label("Open in MDMA"),
                )
                .add_embeds(
                    result
                        .iter()
                        .map(|member| {
                            CreateEmbed::new()
                                .title(format!("{}, {}", member.last_name, member.first_name))
                                .field("email", member.email.clone(), false)
                                .field("created_on", format!("{}", member.created_on), false)
                                .field(
                                    "first_payment",
                                    format!(
                                        "{}",
                                        member.first_payment.unwrap_or(date!(1970 - 01 - 01))
                                    ),
                                    false,
                                )
                                .field(
                                    "consecutive_since",
                                    format!(
                                        "{}",
                                        member.consecutive_since.unwrap_or(date!(1970 - 01 - 01))
                                    ),
                                    true,
                                )
                                .field(
                                    "consecutive_until",
                                    format!(
                                        "{}",
                                        member.consecutive_until.unwrap_or(date!(1970 - 01 - 01))
                                    ),
                                    true,
                                )
                                .field(
                                    "generation_name",
                                    member.generation_name.as_deref().unwrap_or("null"),
                                    false,
                                )
                                .field(
                                    "is_active",
                                    member.is_active.unwrap_or(false).to_string(),
                                    false,
                                )
                        })
                        .collect(),
                ),
        ))
    }
}

struct RegisterUserResponse {
    first_name: String,
    last_name: String,
}

async fn register_user(
    email: &str,
    user_id: UserId,
    assign_role: Option<RoleId>,
    state: crate::AppState,
) -> CreateInteractionResponse {
    match sqlx::query_as!(
        RegisterUserResponse,
        "UPDATE members SET discord=$1 WHERE email=LOWER($2) RETURNING first_name, last_name",
        Decimal::from(user_id.get()),
        email
    )
    .fetch_optional(&state.db_pool)
    .await
    {
        Ok(Some(result)) => {
            if let Some(role_id) = assign_role {
                let _ = state
                    .discord_http
                    .add_member_role(state.discord_guild, user_id, role_id, None)
                    .await;
            }
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Welcome, {} {}! Thank you for joining.",
                        result.first_name, result.last_name
                    ))
                    .ephemeral(true),
            )
        }
        Ok(None) => CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!(
                    "Could not find a member with the email {}. Please try again.",
                    email
                ))
                .ephemeral(true),
        ),
        Err(_) => CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Something went wrong. Please notify an admin and try again later.")
                .ephemeral(true),
        ),
    }
}

fn handle_component_interaction(
    event: ComponentInteraction,
) -> Result<CreateInteractionResponse, StatusCode> {
    match event.data.custom_id.split_once(":") {
        Some(("mdma_open_register_modal", role_id)) => Ok(CreateInteractionResponse::Modal(
            CreateModal::new(format!("mdma_register_modal:{}", role_id), "Accept & Join")
                .components(vec![CreateActionRow::InputText(
                    CreateInputText::new(InputTextStyle::Short, "EMAIL", "mdma_register_email")
                        .placeholder("jon.doe@example.com")
                        .required(true),
                )]),
        )),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_modal_interaction(
    event: ModalInteraction,
    state: crate::AppState,
) -> Result<CreateInteractionResponse, StatusCode> {
    match event.data.custom_id.split_once(":") {
        Some(("mdma_register_modal", role_id)) => match &event.data.components[0].components[0] {
            ActionRowComponent::InputText(textbox) => Ok(register_user(
                textbox.value.as_ref().ok_or(StatusCode::BAD_REQUEST)?,
                event.user.id,
                RoleId::from_str(role_id).ok(),
                state,
            )
            .await),
            &_ => Err(StatusCode::BAD_REQUEST),
        },
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_slash_command(
    event: CommandInteraction,
    state: crate::AppState,
) -> Result<CreateInteractionResponse, StatusCode> {
    let options: std::collections::HashMap<String, CommandDataOptionValue> = event
        .data
        .options
        .iter()
        .map(|o| (o.name.clone(), o.value.clone()))
        .collect();

    match event.data.name.as_str() {
        "register_users" => Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().button(
                CreateButton::new(format!(
                    "mdma_open_register_modal:{}",
                    match options.get("assign_role") {
                        None => String::from("_"),
                        Some(CommandDataOptionValue::Role(role)) => role.get().to_string(),
                        Some(_) => return Err(StatusCode::BAD_REQUEST),
                    }
                ))
                .label("Accept & Join")
                .style(ButtonStyle::Success),
            ),
        )),

        "whois" => {
            whois(
                match options.get("user") {
                    Some(CommandDataOptionValue::User(user)) => *user,
                    _ => return Err(StatusCode::BAD_REQUEST),
                },
                state.secret_store.get("MDMA_URL").unwrap().as_str(),
                &state.db_pool,
            )
            .await
        }

        "MDMA WhoIs User" => {
            whois(
                match event.data.target_id {
                    Some(tid) => tid.to_user_id(),
                    None => return Err(StatusCode::BAD_REQUEST),
                },
                state.secret_store.get("MDMA_URL").unwrap().as_str(),
                &state.db_pool,
            )
            .await
        }

        &_ => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn handle_request(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let signature = headers
        .get("X-Signature-Ed25519")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let timestamp = headers
        .get("X-Signature-Timestamp")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if state
        .discord_verifier
        .verify(signature, timestamp, body.as_ref())
        .is_err()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let response = match serenity::json::from_slice::<Interaction>(body.as_ref())
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        Interaction::Ping(_) => CreateInteractionResponse::Pong,
        Interaction::Component(event) => handle_component_interaction(event)?,
        Interaction::Modal(event) => handle_modal_interaction(event, state).await?,
        Interaction::Command(event) => handle_slash_command(event, state).await?,
        _ => return Err(StatusCode::NOT_IMPLEMENTED),
    };

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serenity::json::to_string(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
        .into_response())
}

pub async fn create_commands(
    crate::AppState {
        discord_http,
        discord_guild,
        ..
    }: &crate::AppState,
) {
    CreateCommand::new("register_users")
        .description("Create a button to register Discord users in MDMA")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(CreateCommandOption::new(
            CommandOptionType::Role,
            "assign_role",
            "Assign a role to members after successfully registering",
        ))
        .execute(&discord_http, (Some(*discord_guild), None))
        .await
        .expect("/register_users");

    CreateCommand::new("whois")
        .description("Lookup a Discord user in MDMA")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "The user to look up")
                .required(true),
        )
        .execute(&discord_http, (Some(*discord_guild), None))
        .await
        .expect("/whois");

    CreateCommand::new("MDMA WhoIs User")
        .kind(CommandType::User)
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .execute(&discord_http, (Some(*discord_guild), None))
        .await
        .expect("user:whois");
}

#[derive(serde::Serialize)]
struct InviteOptions {
    max_age: u64,
    max_uses: u8,
    unique: bool,
}

impl Default for InviteOptions {
    fn default() -> Self {
        Self {
            max_age: 604800,
            max_uses: 1,
            unique: true,
        }
    }
}

pub async fn create_invite(
    reason: Option<&str>,
    state: &crate::AppState,
) -> Result<String, serenity::Error> {
    Ok(state
        .discord_http
        .create_invite(
            state
                .secret_store
                .get("DISCORD_INVITE_CHANNEL_ID")
                .unwrap()
                .parse::<ChannelId>()
                .unwrap(),
            &InviteOptions::default(),
            reason,
        )
        .await?
        .url())
}
