use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPayload {
    pub data: EventDetails,
}

#[derive(Deserialize)]
pub struct Name {
    pub first: String,
    pub last: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Billing {
    pub email: String,
    pub name: Name,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetails {
    pub total: Decimal,
    pub billing: Billing,
    pub transaction_id: i32,
}
