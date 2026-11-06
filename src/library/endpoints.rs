use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tracing::warn;

use super::models::LibraryState;

pub fn create_download_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        "/{book_id}/{book_format}/{filename}",
        get(handle_file_download),
    )
}

async fn handle_file_download(
    Path((book_id, _book_format)): Path<(String, String)>,
    State(library): State<Arc<Mutex<LibraryState>>>,
) -> Response {
    let book = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.find_book_by_id(&book_id).cloned()
    };

    let Some(book) = book else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let data = match tokio::fs::read(&book.path).await {
        Ok(data) => data,
        Err(error) => {
            warn!("cannot read {}: {error}", book.path.display());
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let content_type = match book.format() {
        "KEPUB" => "application/x-kobo-epub+zip",
        _ => "application/epub+zip",
    };

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(content_type),
            ),
            (
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&data.len().to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            ),
        ],
        data,
    )
        .into_response()
}
