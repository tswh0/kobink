use axum::{
    http::StatusCode,
    middleware,
    routing::get,
    Router,
};

mod errors;
pub use errors::AppError;
mod logging;
use crate::{
    auth,
    kobo::{self, create_kobo_router},
    library::{create_download_router, models::LibraryState},
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tracing::info;

pub async fn start_server() {
    let library_path =
        std::env::var("LIBRARY_PATH").unwrap_or_else(|_| "/media/kobink_test".to_string());
    let initial_state = LibraryState::new(&library_path);
    let arcref = Arc::new(Mutex::new(initial_state));

    spawn_library_refresh(Arc::clone(&arcref), &library_path);

    let api_key = auth::api_key();
    let app = Router::<Arc<Mutex<LibraryState>>>::new()
        .route("/", get(root))
        .nest(&format!("/{api_key}/kobo/v1"), create_kobo_router())
        .nest(&format!("/{api_key}/download"), create_download_router())
        .merge(kobo::covers::create_cover_router(&format!("/{api_key}")))
        .fallback(not_found)
        .layer(middleware::from_fn(logging::log_request_response))
        .with_state(Arc::clone(&arcref));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr.to_string())
        .await
        .unwrap();
    info!("listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

fn spawn_library_refresh(library: Arc<Mutex<LibraryState>>, library_path: &str) {
    let interval_seconds = std::env::var("LIBRARY_REFRESH_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(600);

    if interval_seconds == 0 {
        info!("library refresh disabled");
        return;
    }

    let library_path = library_path.to_string();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_seconds));
        ticker.tick().await;
        loop {
            ticker.tick().await;

            let path = library_path.clone();
            let books = match tokio::task::spawn_blocking(move || crate::library::models::scan(&path))
                .await
            {
                Ok(books) => books,
                Err(error) => {
                    tracing::warn!("library scan failed: {error}");
                    continue;
                }
            };

            let count = books.len();
            library
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .books = Arc::new(books);
            info!("library refreshed: {} books", count);
        }
    });

    info!("library refresh every {}s", interval_seconds);
}

async fn root() -> &'static str {
    "Hello, World!"
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
