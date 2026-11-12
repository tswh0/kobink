mod auth;
mod drop;
mod kobo;
mod library;
mod server;

use server::start_server;
use std::{net::TcpStream, time::Duration};
use tracing::info;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--health") => std::process::exit(health()),
        Some("--version") | Some("-V") => {
            println!("kobink {VERSION}");
            return;
        }
        _ => {}
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
    tracing_subscriber::fmt().init();

    info!("kobink {VERSION} says hello");

    auth::log_endpoint();
    start_server().await
}
