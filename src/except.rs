use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;
use std::{str::FromStr, sync::atomic::Ordering};

use event_listener::Event;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};
use zbus::connection;

use crate::challenge::{CHALLENGE_CANCELLED, CHALLENGE_REQUESTED, Challenge};
pub(crate) use crate::dbus::ExceptManager;

const DBUS_NAME: &str = "net.anunknownalias.ExceptManager";
const DBUS_PATH: &str = "/net/anunknownalias/ExceptManager";

pub struct Except {
    ip: std::net::Ipv4Addr,
    port: u16,

    event: Arc<event_listener::Event>,
    tx: tokio::sync::broadcast::Sender<u8>,
    verified: Arc<std::sync::atomic::AtomicBool>,
    dbus: Option<zbus::Connection>,
}

impl Except {
    pub fn new(ip: &str, port: u16) -> Self {
        let ip = std::net::Ipv4Addr::from_str(ip).unwrap();
        let event = Arc::new(Event::new());
        let (tx, _) = tokio::sync::broadcast::channel(2);
        let verified = Arc::new(AtomicBool::new(false));
        Self {
            ip,
            port,
            event,
            tx,
            verified,
            dbus: None,
        }
    }

    pub async fn dbus_connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!(DBUS_NAME, DBUS_PATH, "starting dbus service");
        let dbus = ExceptManager::new(self.event.clone(), self.tx.clone(), self.verified.clone());
        let connection = connection::Builder::session()?
            .name(DBUS_NAME)?
            .serve_at(DBUS_PATH, dbus)?
            .build()
            .await?;

        self.dbus = Some(connection);
        Ok(())
    }

    pub async fn start_listener(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!(
            ip = self.ip.to_string(),
            self.port, "opening the tcp listener"
        );
        let listener = TcpListener::bind((self.ip, self.port)).await?;
        loop {
            let (socket, _) = listener.accept().await?;
            info!("accepted connection from: {}", socket.peer_addr()?.ip());

            let rx = self.tx.subscribe();
            let event = self.event.clone();
            let verified = self.verified.clone();
            debug!("spawning a new client handling task");
            tokio::spawn(async move {
                if let Err(e) = Except::handle_client(socket, rx, event, verified).await {
                    error!("an error occurred; error = {:?}", e);
                }
            });
        }
    }

    async fn handle_client(
        mut stream: TcpStream,
        mut rx: tokio::sync::broadcast::Receiver<u8>,
        event: Arc<event_listener::Event>,
        verified: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0; 1];
        let mut peek_buf = [0; 1];
        stream.read_exact(&mut buf).await?;

        debug!("notifying the dbus manager and waiting for the device id");
        let recv = rx.recv();
        event.notify(1);
        tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    return Err("stream timeout".into());
                },
                _ = stream.peek(&mut peek_buf) => {
                    return Err("invalid stream sequence".into());
                }
                Ok(id) = recv => {
                    Except::client_requests(&buf, id, stream, verified).await?;
                }
        }
        Ok(())
    }

    async fn client_requests(
        buf: &[u8; 1],
        id: u8,
        mut stream: TcpStream,
        verified: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peer = stream.peer_addr()?.to_string();
        match buf[0] {
            CHALLENGE_CANCELLED => {
                verified.store(false, Ordering::Release);
                debug!(peer, "challenge cancelled by the client");
                Ok(())
            }
            CHALLENGE_REQUESTED => {
                debug!(peer, "received challenge request");
                let result = Challenge::run(&mut stream, id, &peer).await?;
                verified.store(result, Ordering::Release);
                debug!(peer, result, "challange completed");

                Ok(())
            }
            _ => Err("invalid request made to tcp stream".into()),
        }
    }
}
