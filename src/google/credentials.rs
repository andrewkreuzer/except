use std::time::{self, Duration};

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;
use tracing::debug;

use super::types::*;

const DEFAULT_TOKEN_EXPIRY: Duration = Duration::from_secs(3600);

pub struct Credentials {
    signer: EncodingKey,
    token: Option<AuthToken>,
    token_header: Header,
    claims: Claims,
    token_uri: String,
    default_headers: reqwest::header::HeaderMap,
    token_expiry: Duration,
}

impl Credentials {
    pub fn from_service_account_file(f: &str) -> Self {
        debug!("loading service account from file: {}", f);
        let file_content = std::fs::read_to_string(f).unwrap();
        let service_account: ServiceAccount = serde_json::from_str(&file_content).unwrap();
        let signer = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes()).unwrap();
        let token_uri = service_account.token_uri.clone();
        let default_headers = Credentials::default_headers();
        let (iat, exp) = Credentials::claim_iat_exp();

        Credentials {
            signer,
            token_uri,
            token: None,
            default_headers,
            token_expiry: DEFAULT_TOKEN_EXPIRY,
            token_header: Header {
                typ: Some("JWT".into()),
                alg: Algorithm::RS256,
                kid: Some(service_account.private_key_id.clone()),
                ..Header::default()
            },
            claims: Claims {
                iat,
                exp,
                iss: service_account.client_email.clone(),
                aud: service_account.token_uri.clone(),
                scope: "https://www.googleapis.com/auth/firebase.messaging".into(),
            },
        }
    }

    #[allow(dead_code)]
    pub fn set_token_expiry(&mut self, exp: Duration) {
        self.token_expiry = exp;
        self.claims.exp = self.claims.iat + exp.as_secs();
    }

    pub fn get_access_token(&self) -> Result<&str, Box<dyn std::error::Error>> {
        match self.token.as_ref() {
            Some(t) => Ok(&t.access_token),
            None => Err("no token found, try refreshing".into()),
        }
    }

    pub async fn refresh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!(token_uri=self.token_uri, "refreshing token");
        let assertion = encode(&self.token_header, &self.claims, &self.signer)?;
        let mut headers = self.default_headers.clone();
        headers.insert("Authorization", format!("Bearer {}", assertion).parse()?);
        self.update_claim_iat_exp()?;

        debug!("sending token refresh request");
        let res = reqwest::Client::new()
            .post(&self.token_uri)
            .headers(headers)
            .body(
                json!({
                    "assertion": assertion,
                    "grant_type": "urn:ietf:params:oauth:grant-type:jwt-bearer"
                })
                .to_string(),
            )
            .send()
            .await?;

        if !res.status().is_success() {
            let msg: Message = res.json().await?;
            return Err(format!("Failed to refresh token: {:?}", msg).into());
        }

        let auth_res = res.json::<AuthToken>().await?;
        self.token = Some(auth_res);

        debug!("token refreshed successfully");
        Ok(())
    }

    fn update_claim_iat_exp(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.claims.iat = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.claims.exp = self.claims.iat + self.token_expiry.as_secs();

        Ok(())
    }

    fn claim_iat_exp() -> (u64, u64) {
        let iat = match time::SystemTime::now().duration_since(time::UNIX_EPOCH) {
            Ok(d) => d,
            Err(e) => panic!("Failed to get system time for issued at claim: {}", e),
        };
        let exp = iat + DEFAULT_TOKEN_EXPIRY;
        (iat.as_secs(), exp.as_secs())
    }

    fn default_headers() -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert(
            "x-goog-api-client",
            "random-project/1.2.3 self-rolled/1.2.3 auth-request-type/at cred-type/sa"
                .parse()
                .unwrap(),
        );
        headers
    }
}
