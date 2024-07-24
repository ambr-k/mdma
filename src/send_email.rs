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

pub fn populate_discord_email(
    values: &EmailValues,
    persist: &PersistInstance,
) -> tinytemplate::error::Result<String> {
    let raw_contents = persist.load::<String>("discord_email").unwrap_or_default();
    let mut templ = TinyTemplate::new();
    templ.add_template("discord_email", &raw_contents)?;
    templ
        .render("discord_email", values)
        .map(|populated| sanitize_email(&populated))
}
