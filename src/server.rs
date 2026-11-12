use axum::{
    http::{header, HeaderValue, StatusCode},
    middleware,
    Router,
};

mod logging;
use crate::{
    auth,
    drop,
    kobo::{self, create_kobo_router},
    library::{
        create_download_router,
        models::{refresh, LibraryState},
        retention,
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
    let library_path = std::env::var("LIBRARY_PATH").unwrap_or_else(|_| "/library".to_string());
    let covers_path =
        std::env::var("COVER_CACHE_DIR").unwrap_or_else(|_| "/app/data/covers".to_string());

    let arcref = Arc::new(Mutex::new(LibraryState::empty(&library_path)));

    spawn_library_refresh(Arc::clone(&arcref));
    spawn_retention(library_path.clone(), covers_path);

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
        .layer(middleware::from_fn(security_headers))
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

    spawn_scan(Arc::clone(&arcref));
    spawn_kcc_check();

    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
    {
        tracing::error!("server stopped: {error}");
        std::process::exit(1);
    }
}

fn spawn_scan(library: Arc<Mutex<LibraryState>>) {
    tokio::spawn(async move {
        refresh(library).await;
    });
}

fn spawn_kcc_check() {
    tokio::task::spawn_blocking(|| {
        info!(
            "kcc: {}",
            if drop::kcc_available() {
                "found, CBZ and PDF are converted for the e-reader"
            } else {
                "missing, only EPUB is accepted"
            }
        );
    });
}

fn spawn_retention(library_path: String, covers_path: String) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));

        loop {
            ticker.tick().await;

            let (library_path, covers_path) = (library_path.clone(), covers_path.clone());

            if tokio::task::spawn_blocking(move || retention::sweep(&library_path, &covers_path))
                .await
                .is_err()
            {
                break;
            }
        }
    });
}

async fn shutdown() {
    let mut terminate = match tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    ) {
        Ok(signal) => signal,
        Err(error) => {
            tracing::warn!("cannot listen for SIGTERM: {error}");

            std::future::pending::<tokio::signal::unix::Signal>().await
        }
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }

    info!("shutting down");
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

async fn security_headers(
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let kobo = request.uri().path().contains("/kobo/v1/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    if kobo {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
             connect-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );

    response
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
