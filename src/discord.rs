use std::str::FromStr;

use askama_axum::IntoResponse;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use reqwest::header;
use rust_decimal::Decimal;
use serenity::{builder::*, model::prelude::*};

async fn register_user(
    email: &str,
    user_id: UserId,
    assign_role: Option<RoleId>,
    state: crate::AppState,
) -> CreateInteractionResponse {
    match sqlx::query!(
        "UPDATE members SET discord=$1 WHERE email=LOWER($2)",
        Decimal::from(user_id.get()),
        email
    )
    .execute(&state.db_pool)
    .await
    {
        Ok(result) => CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(if result.rows_affected() == 1 {
                    if let Some(role_id) = assign_role {
                        let _ = state
                            .discord_http
                            .add_member_role(state.discord_guild, user_id, role_id, None)
                            .await;
                    }
                    String::from("Thank you for joining!")
                } else {
                    format!(
                        "Could not find a member with the email {}. Please try again.",
                        email
                    )
                })
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

fn handle_slash_command(
    event: CommandInteraction,
) -> Result<CreateInteractionResponse, StatusCode> {
    let options: std::collections::HashMap<String, CommandDataOptionValue> = event
        .data
        .options
        .iter()
        .map(|o| (o.name.clone(), o.value.clone()))
        .collect();

    let role_id = match options.get("assign_role") {
        None => String::from("_"),
        Some(CommandDataOptionValue::Role(role)) => role.get().to_string(),
        Some(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match event.data.name.as_str() {
        "register_users" => Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().button(
                CreateButton::new(format!("mdma_open_register_modal:{}", role_id))
                    .label("Accept & Join"),
            ),
        )),
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
        Interaction::Command(event) => handle_slash_command(event)?,
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
        .unwrap();
}
