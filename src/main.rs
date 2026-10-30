mod auth;
mod kobo;
mod library;
mod server;

use dotenvy::dotenv;
use server::start_server;

#[tokio::main]
async fn main() {
    let _ = dotenv();
    tracing_subscriber::fmt().init();

    auth::log_endpoint();
    start_server().await
}
