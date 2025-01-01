use std::error::Error;

use ring::{
    aead::{Aad, BoundKey, Nonce, NonceSequence},
    error::Unspecified,
    rand::{SecureRandom, SystemRandom},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

pub const CHALLENGE_REQUESTED: u8 = 80;
const CHALLENGE_ACCEPTED: u8 = 82;
const CHALLENGE_APPROVED: u8 = 65;
const CHALLENGE_REJECTED: u8 = 83;
pub const CHALLENGE_CANCELLED: u8 = 127;
const EOF: &[u8] = &[0; 4];

const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
const FUNC1: fn(u8, u8) -> u8 = |op: u8, x: u8| x.wrapping_mul(op);

pub(crate) struct Challenge {
    id: u8,
    data: Vec<u8>,
    op: u8,
    response: Option<Response>,
    key: &'static [u8; 32],
}

impl Challenge {
    fn with_fn(f: fn(u8, u8) -> u8, id: u8) -> Self {
        let mut data = [0; 5];
        SystemRandom::new().fill(&mut data).unwrap();
        let op = data[0];
        let data = data[1..].to_vec();
        let response = Some(Response::new(&data, op, f));
        Self {
            id,
            data,
            op,
            response,
            key: KEY,
        }
    }

    async fn write_to_buf(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buf = vec![];
        buf.extend(&[self.id, self.op]);
        buf.extend(&self.data);
        buf.extend(EOF);
        Ok(buf)
    }

    async fn encrypt_data(&mut self) -> Result<(), Box<dyn Error>> {
        let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, self.key).unwrap();
        let mut sealing_key = ring::aead::SealingKey::new(unbound_key, NONCE_GEN);
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut self.data)
            .unwrap();

        Ok(())
    }

    #[allow(dead_code)]
    async fn decrypt_data(&mut self) -> Result<(), Box<dyn Error>> {
        let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, self.key).unwrap();
        let mut opening_key = ring::aead::OpeningKey::new(unbound_key, NONCE_GEN);
        opening_key
            .open_in_place(Aad::empty(), &mut self.data)
            .unwrap();
        Ok(())
    }

    async fn verify(&self, got: &[u8]) -> Result<(), Box<dyn Error>> {
        if let Some(response) = self.response.as_ref() {
            for (i, n) in response.data.iter().enumerate() {
                if got[i] != *n {
                    return Err("invalid response".into());
                }
            }
        }
        Ok(())
    }

    async fn decrypt(buf: &mut [u8]) -> Result<&[u8], Box<dyn Error>> {
        let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, KEY).unwrap();
        let mut opening_key = ring::aead::OpeningKey::new(unbound_key, NONCE_GEN);
        match opening_key.open_in_place(Aad::empty(), buf) {
            Ok(in_out) => Ok(in_out),
            Err(_) => Err("error decrypting buffer".into()),
        }
    }

    async fn read_until(
        stream: &mut TcpStream,
        until: &[u8],
    ) -> Result<(Vec<u8>, usize), Box<dyn Error>> {
        let mut length = 0;
        let mut res_buf = vec![0; 64];
        loop {
            length += stream.read(&mut res_buf).await?;
            if res_buf[..length].ends_with(until) {
                length -= until.len();
                break;
            }
        }

        Ok((res_buf, length))
    }

    pub async fn run(stream: &mut TcpStream, id: u8, peer: &str) -> Result<bool, Box<dyn Error>> {
        debug!(peer, "generating and encrypting the challenge");
        let mut challenge = Challenge::with_fn(FUNC1, id);

        debug!(peer, "encrypting and sending the challenge");
        challenge.encrypt_data().await?;
        let buf = challenge.write_to_buf().await?;
        stream.write_all(&buf).await?;

        let mut accepted = [0; 1];
        stream.read_exact(&mut accepted).await?;
        match accepted {
            [CHALLENGE_ACCEPTED] => info!(peer, "challenge has been accepted"),
            _ => {
                info!(peer, "challenge has been rejected");
                return Err("challenge rejected".into());
            }
        }

        debug!(peer, "receiving and decrypting the challenge response");
        let (mut buf, length) = Challenge::read_until(stream, EOF).await?;
        let plaintext = Challenge::decrypt(&mut buf[..length]).await?;

        debug!(peer, "verifying the challenge response");
        let result = match challenge.verify(plaintext).await {
            Ok(_) => {
                debug!(peer, "challenge response verified, sending approval");
                (&[CHALLENGE_APPROVED], true)
            }
            Err(e) => {
                error!("{}", e);
                (&[CHALLENGE_REJECTED], false)
            }
        };
        stream.write_all(result.0).await?;
        Ok(result.1)
    }

    // kept for reference
    #[allow(dead_code)]
    pub async fn call() -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect("192.168.2.106:6667").await?;
        stream.write_all(&[CHALLENGE_REQUESTED]).await?;

        let mut challenge_buf = [0; 32];
        let mut length = 0;
        let mut reading = true;
        while reading {
            length += stream.read(&mut challenge_buf).await?;
            if challenge_buf[..length].ends_with(EOF) {
                length -= EOF.len();
                reading = false;
            }
        }
        stream.write_all(&[CHALLENGE_ACCEPTED]).await?;

        let mut challenge = Challenge::from(&challenge_buf[..length]);
        challenge.decrypt_data().await?;

        let response = Response::new(&challenge.data, challenge.op, FUNC1);
        let mut response_data = response.data.clone();
        let unbound_key = ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, KEY).unwrap();
        let mut sealing_key = ring::aead::SealingKey::new(unbound_key, NONCE_GEN);
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut response_data)
            .unwrap();
        response_data.extend(EOF);
        stream.write_all(&response_data).await?;

        let mut response = [0; 1];
        stream.read_exact(&mut response).await?;

        if response[0] == CHALLENGE_APPROVED {
            info!("Challenge approved by: {}", stream.peer_addr()?.ip());
        } else {
            info!("Challenge rejected");
        }

        Ok(())
    }
}

impl From<&[u8]> for Challenge {
    fn from(payload: &[u8]) -> Self {
        let _id = payload[0];
        let op = payload[1];
        let data = payload[2..].to_vec();
        Self {
            id: 0,
            data,
            op,
            response: None,
            key: KEY,
        }
    }
}

struct Response {
    data: Vec<u8>,
}

impl Response {
    fn new(data: &[u8], op: u8, f: fn(u8, u8) -> u8) -> Self {
        Self {
            data: data.iter().map(|x| f(op, *x)).collect(),
        }
    }
}

struct NonceGenerator {
    counter: [u8; 12],
}

impl NonceSequence for NonceGenerator {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        // self.counter[0] = self.counter[0].wrapping_add(1);
        Ok(Nonce::assume_unique_for_key(self.counter))
    }
}

const NONCE_GEN: NonceGenerator = NonceGenerator { counter: [0u8; 12] };
