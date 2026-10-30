use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use epub::doc::EpubDoc;

use crate::library::models::LibraryState;

pub fn create_cover_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        &format!("{prefix}/:book_id/:width/:height/*image"),
        get(handle_cover),
    )
}

async fn handle_cover(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path((book_id, _, _, _)): Path<(String, String, String, String)>,
) -> Response {
    let path = {
        let library = library.lock().unwrap();
        library.find_book_by_id(&book_id).map(|book| book.path.clone())
    };

    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let cover = tokio::task::spawn_blocking(move || {
        let mut doc = EpubDoc::new(path).ok()?;
        doc.get_cover()
    })
    .await
    .ok()
    .flatten();

    match cover {
        Some((data, mime)) => ([(header::CONTENT_TYPE, mime)], data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
