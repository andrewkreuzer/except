use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FCMOptions {
    pub analytics_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Priority {
    #[serde(rename = "NORMAL")]
    Normal,
    #[serde(rename = "HIGH")]
    High,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AndroidConfig {
    pub collapse_key: Option<String>,
    pub priority: Option<Priority>,
    pub ttl: Option<String>,
    pub restricted_package_name: Option<String>,
    pub data: Option<HashMap<String, String>>,
    pub notification: Option<Notification>,
    pub fcm_options: Option<FCMOptions>,
    pub direct_boot_ok: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub name: Option<String>,
    pub token: Option<String>,
    pub notification: Option<Notification>,
    pub android: Option<AndroidConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FCMMessage {
    validate_only: Option<bool>,
    pub message: Message,
}

impl TryFrom<String> for FCMMessage {
    type Error = Box<dyn std::error::Error>;
    fn try_from(s: String) -> Result<Self, Box<dyn std::error::Error>> {
        let msg: FCMMessage = serde_json::from_str(&s)?;
        Ok(msg)
    }
}

impl TryFrom<&str> for FCMMessage {
    type Error = Box<dyn std::error::Error>;
    fn try_from(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let msg: FCMMessage = serde_json::from_str(s)?;
        Ok(msg)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ServiceAccount {
    pub(crate) token_uri: String,
    pub(crate) private_key_id: String,
    pub(crate) private_key: String,
    pub(crate) client_email: String,

    #[serde(rename = "type")]
    _type: String,
    project_id: String,
    client_id: String,
    auth_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
    universe_domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    pub(crate) iat: u64,
    pub(crate) exp: u64,
    pub(crate) iss: String,
    pub(crate) aud: String,
    pub(crate) scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthToken {
    pub(crate) access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
enum Status {
    #[serde(rename = "UNSPECIFIED_ERROR")]
    UnspecifiedError,
    #[serde(rename = "INVALID_ARGUMENT")]
    InvalidArgument,
    #[serde(rename = "UNREGISTERED")]
    Unregistered,
    #[serde(rename = "SENDER_ID_MISMATCH")]
    SenderIdMismatch,
    #[serde(rename = "QUOTA_EXCEEDED")]
    QuotaExceeded,
    #[serde(rename = "UNAVAILABLE")]
    Unavailable,
    #[serde(rename = "INTERNAL")]
    Internal,
    #[serde(rename = "THIRD_PARTY_AUTH_ERROR")]
    ThirdPartyAuthError,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldViolations {
    field: String,
    description: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorDetails {
    #[serde(rename = "@type")]
    _type: String,
    field_violations: Option<Vec<FieldViolations>>,
    error_code: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct FCMInnerError {
    code: i32,
    message: String,
    status: Status,
    details: Vec<ErrorDetails>
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FCMError {
    #[serde(rename = "error")]
    inner: FCMInnerError
}

impl std::error::Error for FCMError {}

impl std::fmt::Display for FCMError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.inner.message)
    }
}

