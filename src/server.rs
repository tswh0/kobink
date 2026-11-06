use axum::{http::StatusCode, middleware, Router};

mod logging;
use crate::{
    auth,
    drop,
    kobo::{self, create_kobo_router},
    library::{
        create_download_router,
        models::{refresh, LibraryState},
    },
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::compression::CompressionLayer;
use tracing::info;

pub fn port() -> u16 {
    std::env::var("KOBINK_PORT")
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .unwrap_or(3000)
}

pub async fn start_server() {
    let library_path =
        std::env::var("LIBRARY_PATH").unwrap_or_else(|_| "/library".to_string());
    let initial_state = LibraryState::new(&library_path);
    let arcref = Arc::new(Mutex::new(initial_state));

    spawn_library_refresh(Arc::clone(&arcref));

    let api_key = auth::api_key();
    let app = Router::<Arc<Mutex<LibraryState>>>::new()
        .nest(
            &format!("/{api_key}/kobo/v1"),
            create_kobo_router().layer(CompressionLayer::new()),
        )
        .nest(&format!("/{api_key}/download"), create_download_router())
        .merge(kobo::covers::create_cover_router(&format!("/{api_key}")))
        .merge(drop::create_drop_router(&format!("/{api_key}")))
        .fallback(not_found)
        .layer(middleware::from_fn(logging::log_request_response))
        .with_state(Arc::clone(&arcref));

    let addr = SocketAddr::from(([0, 0, 0, 0], port()));
    let listener = match tokio::net::TcpListener::bind(addr.to_string()).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!("cannot listen on {addr}: {error}");
            std::process::exit(1);
        }
    };

    info!("listening on {}", addr);

    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!("server stopped: {error}");
        std::process::exit(1);
    }
}

fn spawn_library_refresh(library: Arc<Mutex<LibraryState>>) {
    let interval_seconds = std::env::var("LIBRARY_REFRESH_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(600);

    if interval_seconds == 0 {
        info!("library refresh disabled");
        return;
    }

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_seconds));
        ticker.tick().await;

        loop {
            ticker.tick().await;
            refresh(Arc::clone(&library)).await;
        }
    });

    info!("library refresh every {}s", interval_seconds);
}

async fn not_found(uri: axum::http::Uri) -> (StatusCode, String) {
    let path = uri.path();

    if path.contains("/v1/v1/") {
        return (
            StatusCode::NOT_FOUND,
            format!(
                "no route for {path}: api_endpoint must end at /kobo, the device appends /v1 itself"
            ),
        );
    }

    (StatusCode::NOT_FOUND, format!("no route for {path}"))
}
