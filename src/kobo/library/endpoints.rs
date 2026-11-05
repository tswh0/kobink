use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::{kobo::models::SyncTokenModel, library::models::LibraryState, server::AppError};

use super::models::{Entitlement, SyncEvent, SyncResponse};

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
        let library = library.lock().unwrap();
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
    _headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap, Json<SyncResponse>), AppError> {
    let books = library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .books
        .clone();

    let events: Vec<SyncEvent> = books.iter().map(SyncEvent::new).collect();

    let mut headers = HeaderMap::new();
    if let Ok(token) = HeaderValue::try_from(SyncTokenModel::now().to_header_value()) {
        headers.insert("x-kobo-synctoken", token);
    }

    Ok((StatusCode::OK, headers, Json(events)))
}
