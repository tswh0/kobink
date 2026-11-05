use std::sync::{Arc, Mutex};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::{
    kobo::models::SyncToken,
    library::models::{Book, LibraryState},
};

use super::models::{Entitlement, SyncEvent};

static SYNC_CACHE: Mutex<Option<(u64, Bytes)>> = Mutex::new(None);

pub fn create_library_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/:book_id", delete(stub))
        .route("/:book_id/state", get(handle_state).put(stub))
        .route("/:book_id/metadata", get(handle_metadata))
        .route("/tags/:book_or_tag_id_idk/items", post(stub))
        .route("/tags/:book_or_tag_id_idk/items/delete", post(stub))
        .route("/sync", get(handle_sync))
}

async fn handle_state() -> (StatusCode, &'static str) {
    (StatusCode::OK, "{\"ReadingStates\":[]}")
}

async fn handle_metadata(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path(book_id): Path<String>,
) -> Response {
    let metadata = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library
            .find_book_by_id(&book_id)
            .map(|book| Entitlement::new(book).BookMetadata)
    };

    let Some(metadata) = metadata else {
        return StatusCode::NOT_FOUND.into_response();
    };

    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        Json(vec![metadata]),
    )
        .into_response()
}

async fn stub() -> StatusCode {
    StatusCode::OK
}

async fn handle_sync(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
) -> Response {
    let (version, books) = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (library.version, library.books.clone())
    };

    let known = headers
        .get("x-kobo-synctoken")
        .and_then(|value| value.to_str().ok())
        .and_then(SyncToken::decode);

    let body = match known {
        Some(token) if token.library == version => Bytes::from_static(b"[]"),
        _ => sync_body(version, &books),
    };

    let mut response_headers = HeaderMap::new();
    if let Ok(token) = HeaderValue::try_from(SyncToken::new(version).encode()) {
        response_headers.insert("x-kobo-synctoken", token);
    }
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );

    (StatusCode::OK, response_headers, body).into_response()
}

fn sync_body(version: u64, books: &[Book]) -> Bytes {
    let mut cache = SYNC_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some((cached_version, body)) = cache.as_ref() {
        if *cached_version == version {
            return body.clone();
        }
    }

    let events: Vec<SyncEvent> = books.iter().map(SyncEvent::new).collect();
    let body = Bytes::from(serde_json::to_vec(&events).unwrap_or_default());
    *cache = Some((version, body.clone()));

    body
}
