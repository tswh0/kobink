mod auth;
mod kobo;
mod library;
mod server;

use dotenvy::dotenv;
use server::start_server;
use std::{net::TcpStream, time::Duration};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--health") {
        std::process::exit(health());
    }

    run();
}

fn health() -> i32 {
    let address = ([127, 0, 0, 1], server::port()).into();

    match TcpStream::connect_timeout(&address, Duration::from_secs(2)) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[tokio::main]
async fn run() {
    let _ = dotenv();
    tracing_subscriber::fmt().init();

    auth::log_endpoint();
    start_server().await
}
