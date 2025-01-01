use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use event_listener::Listener;
use rand::prelude::*;
use tokio::sync::broadcast::Sender;
use tracing::debug;
use zbus::interface;

use crate::google::{Credentials, FCMMessage, send_message};

pub(crate) struct ExceptManager {
    hostname: String,
    active_id: Option<u8>,
    active_id_verified: Arc<AtomicBool>,
    event: Arc<event_listener::Event>,
    tx: Sender<u8>,
    google_creds: Credentials,
}

impl ExceptManager {
    pub(crate) fn new(
        event: Arc<event_listener::Event>,
        tx: Sender<u8>,
        verified: Arc<AtomicBool>,
    ) -> Self {
        let hostname = std::fs::read_to_string("/etc/hostname").unwrap();
        let hostname = hostname.trim().to_string();
        let active_id_verified = verified;
        let google_creds = Credentials::from_service_account_file("except.json");
        Self {
            hostname,
            active_id: None,
            active_id_verified,
            event,
            tx,
            google_creds,
        }
    }

    async fn firebase_send_auth_notification(
        &mut self,
        id: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.google_creds.refresh().await?;
        let message = auth_notification(id, &self.hostname)?;
        let token = self.google_creds.get_access_token()?;
        send_message(token, message).await
    }
}

#[rustfmt::skip]
fn auth_notification(id: u8, hostname: &str) -> Result<FCMMessage, Box<dyn std::error::Error>> {
    format!(r#"
{{
    "message": {{
        "token": "f3Aff7-AQ6Wid_UFfd15RQ:APA91bFtxN49M-Y_dhDKnG8m_0YBv7K9CGD-SBA6O0V8Ls75mYayRlGTC3vuRs9rU_D6R8bF-M0QMXifrcoR8xZ7fIETdgxmqFsQ_qzGIwNU83jCsclFOPs",
        "android": {{
            "priority": "HIGH",
            "data": {{
                "id": "{id}",
                "device": "{hostname}"
            }}
        }}
    }}
}}"#).trim().try_into()
}

/*
 * Manager->GetDevices
 * Device->Claim
 * # get properties #
 * Device->scan-type
 * Device->name
 * Device->VerifyStatus
 * Device->VerifyFingerSelected
 * Device->VerifyStart
 * Device->VerifyStop
*/

#[interface(
    name = "net.anunknownalias.ExceptManager",
    proxy(
        default_path = "/net/anunknownalias/ExceptManager",
        default_service = "net.anunknownalias.ExceptManager",
    )
)]
impl ExceptManager {
    // TODO: enrollment
    async fn get_default_device(&self) -> u8 {
        let mut rng = rand::thread_rng();
        rng.r#gen()
    }

    async fn start_verify(&mut self, id: u8) -> String {
        debug!(id, "starting Auth flow");
        if let Err(e) = self.firebase_send_auth_notification(id).await {
            return format!("failed to send auth notification: {}", e);
        }
        self.active_id = Some(id);
        let listener = self.event.listen();
        listener.wait_timeout(Duration::from_secs(30));
        let _ = self.tx.send(id);
        debug!(id, "sent id to challenge manager for verification");
        format!("started auth flow for: {}", id)
    }

    async fn verify_status(&self) -> bool {
        let status = self.active_id_verified.load(Ordering::Acquire);
        debug!(status, "status verification check");
        status
    }

    async fn stop_verify(&mut self) {
        self.active_id = None;
        let active_id_verified = false;
        self.active_id_verified
            .store(active_id_verified, Ordering::Release);
        debug!(
            active_id = self.active_id,
            active_id_verified, "auth flow stopped and state reset to"
        );
    }
}
