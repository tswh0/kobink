use std::sync::{Arc, Mutex};

use axum::{
    debug_handler,
    extract::{Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower_http::services::ServeFile;

use super::models::LibraryState;

pub fn create_download_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        "/{book_id}/{book_format}/{filename}",
        get(handle_file_download),
    )
}

#[debug_handler]
async fn handle_file_download(
    Path((book_id, _book_format)): Path<(String, String)>,
    State(library): State<Arc<Mutex<LibraryState>>>,
    request: Request,
) -> Response {
    let book = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.find_book_by_id(&book_id).cloned()
    };

    let Some(book) = book else {
        return (StatusCode::NOT_FOUND, "book not found").into_response();
    };

    let mut file = ServeFile::new(&book.path);
    let mut response = match file.try_call(request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!("cannot serve {}: {error}", book.path.display());

            let status = if error.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            return status.into_response();
        }
    };

    if response.status().is_success() {
        let content_type = match book.format() {
            "KEPUB" => "application/x-kobo-epub+zip",
            _ => "application/epub+zip",
        };
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }

    response.into_response()
}
