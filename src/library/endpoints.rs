use std::sync::{Arc, Mutex};

use axum::{
    debug_handler,
    extract::{Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower::ServiceExt;
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

    let format = book.format();
    let mut response = ServeFile::new(&book.path)
        .oneshot(request)
        .await
        .into_response();

    if response.status().is_success() {
        let content_type = match format {
            "KEPUB" => "application/x-kobo-epub+zip",
            _ => "application/epub+zip",
        };
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }

    response
}
