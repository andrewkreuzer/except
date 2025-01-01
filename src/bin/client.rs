use except::pam_client;

#[tokio::main]
async fn main() {
    match pam_client() {
        Ok(_) => (),
        Err(e) => println!("Error: {:?}", e),
    }
}
