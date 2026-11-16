use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};

use flate2::{write::GzEncoder, Compression};
use tracing::{debug, info, warn};

use crate::{
    kobo::models::SyncToken,
    library::{
        models::{refresh, retire, Book, LibraryState},
        progress,
    },
};

use super::models::{Entitlement, ReadingState, SyncEvent};

static SYNC_CACHE: Mutex<Option<Cached>> = Mutex::new(None);

const SYNCTOKEN_HEADER: &str = "x-kobo-synctoken";

struct Cached {
    version: u64,
    state: u64,
    json: Bytes,
    gzip: Bytes,
}

pub fn create_library_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/{book_id}", delete(handle_delete))
        .route("/{book_id}/state", get(handle_state).put(handle_book_state))
        .route("/{book_id}/metadata", get(handle_metadata))
        .route("/tags/{book_or_tag_id}/items", post(stub))
        .route("/tags/{book_or_tag_id}/items/delete", post(stub))
        .route("/sync", get(handle_sync))
}

async fn handle_state(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path(book_id): Path<String>,
) -> Response {
    let (found, library_path) = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (
            state.find_book_by_id(&book_id).map(|book| book.id),
            state.path.clone(),
        )
    };

    let Some(id) = found else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let states = progress::load(&library_path, &book_id)
        .map(|progress| ReadingState::new(id, &progress))
        .into_iter()
        .collect();

    Json(super::models::ReadingStates { ReadingStates: states }).into_response()
}

async fn handle_book_state(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path(book_id): Path<String>,
    body: Bytes,
) -> Response {
    let (found, library_path) = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (
            state.find_book_by_id(&book_id).is_some(),
            state.path.clone(),
        )
    };

    if !found {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let state = value
        .get("ReadingStates")
        .and_then(|states| states.as_array())
        .and_then(|states| states.first())
        .cloned()
        .unwrap_or(value);

    let text = |pointer: &str| {
        state
            .pointer(pointer)
            .and_then(|field| field.as_str())
            .unwrap_or_default()
    };
    let number = |pointer: &str| state.pointer(pointer).and_then(|field| field.as_f64());

    let percent = number("/CurrentBookmark/ProgressPercent").unwrap_or_default();
    let chapter = text("/CurrentBookmark/Location/Source").to_string();
    let span = text("/CurrentBookmark/Location/Value").to_string();
    let status = text("/StatusInfo/Status");

    let stored = progress::load(&library_path, &book_id);
    let fallback = stored.clone().unwrap_or_default();

    let incoming = progress::Progress {
        percent,
        chapter: if chapter.is_empty() {
            fallback.chapter.clone()
        } else {
            chapter.clone()
        },
        chapter_percent: number("/CurrentBookmark/ContentSourceProgressPercent")
            .unwrap_or(fallback.chapter_percent),
        status: status.to_string(),
        span: if span.is_empty() {
            fallback.span.clone()
        } else {
            span.clone()
        },
        ..Default::default()
    };

    let mut progress = progress::merge(stored, incoming);

    if let Err(error) = progress::save(&library_path, &book_id, &mut progress) {
        warn!("cannot store the reading state of {book_id}: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    info!(
        "reading state from the device: {book_id} {percent:.1}% {chapter}{}",
        if span.is_empty() {
            String::new()
        } else {
            format!(" #{span}")
        }
    );

    StatusCode::OK.into_response()
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

    match metadata {
        Some(metadata) => Json(vec![metadata]).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_delete(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path(book_id): Path<String>,
) -> Response {
    let (found, library_path) = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (
            state.find_book_by_id(&book_id).map(|book| book.path.clone()),
            state.path.clone(),
        )
    };

    let Some(path) = found else {
        debug!("delete for unknown book {book_id}");
        return StatusCode::NO_CONTENT.into_response();
    };

    match retire(&library_path, &path).await {
        Ok(None) => info!("deleted on device: {}", path.display()),
        Ok(Some(target)) => info!("deleted on device: {} -> {}", path.display(), target.display()),
        Err(error) => {
            warn!("cannot delete {}: {error}", path.display());
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    progress::remove(&library_path, &book_id);

    refresh(library).await;
    StatusCode::NO_CONTENT.into_response()
}

async fn stub() -> StatusCode {
    StatusCode::OK
}

async fn handle_sync(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
) -> Response {
    let (version, books, library_path) = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (library.version, library.books.clone(), library.path.clone())
    };

    let state = progress::generation();

    let known = headers
        .get(SYNCTOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(SyncToken::decode);

    let (body, encoded) = match known {
        Some(token) if token.current(version, state) => (Bytes::from_static(b"[]"), false),
        _ => sync_body(version, state, &books, &library_path, accepts_gzip(&headers)),
    };

    let mut response_headers = HeaderMap::new();
    if let Ok(token) = HeaderValue::try_from(SyncToken::new(version, state).encode()) {
        response_headers.insert(SYNCTOKEN_HEADER, token);
    }
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );

    if encoded {
        response_headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        response_headers.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    }

    (StatusCode::OK, response_headers, body).into_response()
}

fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("gzip"))
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    let _ = encoder.write_all(data);

    encoder.finish().unwrap_or_default()
}

fn sync_body(
    version: u64,
    state: u64,
    books: &[Book],
    library_path: &str,
    compressed: bool,
) -> (Bytes, bool) {
    {
        let cache = SYNC_CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(cached) = cache
            .as_ref()
            .filter(|cached| cached.version == version && cached.state == state)
        {
            return if compressed {
                (cached.gzip.clone(), true)
            } else {
                (cached.json.clone(), false)
            };
        }
    }

    let states = progress::all(library_path);

    let events: Vec<SyncEvent> = books
        .iter()
        .map(|book| {
            let state = states
                .get(&book.id.to_string())
                .map(|progress| ReadingState::new(book.id, progress));

            SyncEvent::new(book, state)
        })
        .collect();
    let json = Bytes::from(serde_json::to_vec(&events).unwrap_or_default());
    let zipped = Bytes::from(gzip(&json));

    *SYNC_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Cached {
        version,
        state,
        json: json.clone(),
        gzip: zipped.clone(),
    });

    if compressed {
        (zipped, true)
    } else {
        (json, false)
    }
}
