use tracing::error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut except = except::Except::new("0.0.0.0", 6667);
    if let Err(e) = except.dbus_connect().await {
        error!("Error: {:?}", e);
        std::process::exit(1)
    }
    match except.start_listener().await {
        Ok(_) => (),
        Err(e) => {
            error!("Error: {:?}", e);
        }
    }
}
