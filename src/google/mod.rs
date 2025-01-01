mod credentials;
pub use credentials::Credentials;
mod types;
use tracing::debug;
pub use types::*;

pub async fn send_message(token: &str, msg: FCMMessage) -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://fcm.googleapis.com/v1/projects/persephone-b9fbe/messages:send";
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()?);
    headers.insert("Authorization", format!("Bearer {}", token).parse()?);

    debug!(
        device_token = msg.message.token,
        "sending fcm message"
    );
    let body = serde_json::to_string(&msg)?;
    let res = reqwest::Client::new()
        .post(url)
        .headers(headers)
        .body(body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err = res.json::<FCMError>().await?;
        return Err(Box::new(err));
    }

    let res: Message = res.json().await?;
    debug!(
        msg_id = res.name.as_deref().unwrap_or("no name in response message"),
        "sent fcm notification successfully"
    );

    Ok(())
}
