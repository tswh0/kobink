use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
use tracing::warn;

use super::models::LibraryState;

const STREAM_BUFFER: usize = 256 * 1024;

fn attachment_name(name: &str) -> String {
    name.chars()
        .map(|character| match character {
            '"' | '\\' | ';' | '\r' | '\n' => '_',
            other if other.is_ascii() && !other.is_control() => other,
            _ => '_',
        })
        .collect()
}

pub fn create_download_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        "/{book_id}/{book_format}/{filename}",
        get(handle_file_download),
    )
}

enum Requested {
    Full,
    Part(u64, u64),
    Invalid,
}

fn requested(headers: &HeaderMap, size: u64) -> Requested {
    if size == 0 {
        return Requested::Full;
    }

    let Some(value) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    else {
        return Requested::Full;
    };

    let Some(spec) = value.trim().strip_prefix("bytes=") else {
        return Requested::Full;
    };

    if spec.contains(',') {
        return Requested::Full;
    }

    let Some((start, end)) = spec.split_once('-') else {
        return Requested::Full;
    };

    let (start, end) = (start.trim(), end.trim());

    if start.is_empty() {
        let Ok(suffix) = end.parse::<u64>() else {
            return Requested::Full;
        };

        if suffix == 0 {
            return Requested::Invalid;
        }

        return Requested::Part(size.saturating_sub(suffix), size - 1);
    }

    let Ok(start) = start.parse::<u64>() else {
        return Requested::Full;
    };

    if start >= size {
        return Requested::Invalid;
    }

    let end = if end.is_empty() {
        size - 1
    } else {
        match end.parse::<u64>() {
            Ok(end) => end.min(size - 1),
            Err(_) => return Requested::Full,
        }
    };

    if end < start {
        return Requested::Invalid;
    }

    Requested::Part(start, end)
}

async fn handle_file_download(
    Path((book_id, _book_format, filename)): Path<(String, String, String)>,
    headers: HeaderMap,
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

    let size = match tokio::fs::metadata(&book.path).await {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            warn!("cannot stat {}: {error}", book.path.display());
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let (start, end, partial) = match requested(&headers, size) {
        Requested::Full => (0, size.saturating_sub(1), false),
        Requested::Part(start, end) => (start, end, true),
        Requested::Invalid => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{size}"))],
            )
                .into_response();
        }
    };

    let length = (end + 1).saturating_sub(start);

    let mut file = match tokio::fs::File::open(&book.path).await {
        Ok(file) => file,
        Err(error) => {
            warn!("cannot read {}: {error}", book.path.display());
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    if start > 0 {
        if let Err(error) = file.seek(SeekFrom::Start(start)).await {
            warn!("cannot seek {}: {error}", book.path.display());
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let content_type = match book.format() {
        "KEPUB" => "application/x-kobo-epub+zip",
        _ => "application/epub+zip",
    };

    let mut response = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, length.to_string())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", attachment_name(&filename)),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-store");

    if partial {
        response = response.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }

    let body = Body::from_stream(ReaderStream::with_capacity(
        file.take(length),
        STREAM_BUFFER,
    ));

    response
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
