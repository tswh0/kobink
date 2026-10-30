use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use epub::doc::EpubDoc;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ExtendedColorType};
use tracing::warn;

use crate::library::models::LibraryState;

const DEFAULT_DIMENSION: u32 = 600;
const MAX_DIMENSION: u32 = 3000;
const JPEG_QUALITY: u8 = 80;

pub fn create_cover_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        &format!("{prefix}/:book_id/:width/:height/*image"),
        get(handle_cover),
    )
}

async fn handle_cover(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path((book_id, width, height, _)): Path<(String, String, String, String)>,
) -> Response {
    let path = {
        let library = library.lock().unwrap();
        library.find_book_by_id(&book_id).map(|book| book.path.clone())
    };

    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let width = dimension(&width);
    let height = dimension(&height);
    let id = book_id.clone();

    let rendered = tokio::task::spawn_blocking(move || cover(&id, path, width, height))
        .await
        .ok()
        .flatten();

    match rendered {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=31536000, immutable",
                ),
            ],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn dimension(value: &str) -> u32 {
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_DIMENSION)
        .min(MAX_DIMENSION)
}

fn cover(book_id: &str, path: PathBuf, width: u32, height: u32) -> Option<Vec<u8>> {
    let cached = cache_path(book_id, width, height);

    if let Some(bytes) = cached.as_ref().and_then(|path| fs::read(path).ok()) {
        return Some(bytes);
    }

    let mut doc = EpubDoc::new(&path).ok()?;
    let (data, _) = doc.get_cover()?;

    let decoded = match image::load_from_memory(&data) {
        Ok(decoded) => decoded,
        Err(error) => {
            warn!("cannot decode the cover of {}: {error}", path.display());
            return None;
        }
    };

    let fitted = if decoded.width() <= width && decoded.height() <= height {
        decoded
    } else {
        decoded.resize(width, height, FilterType::Triangle)
    };

    let rgb = fitted.to_rgb8();
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .ok()?;

    if let Some(path) = cached {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, &bytes);
    }

    Some(bytes)
}

fn cache_path(book_id: &str, width: u32, height: u32) -> Option<PathBuf> {
    let dir = std::env::var("COVER_CACHE_DIR").unwrap_or_else(|_| "/app/data/covers".to_string());

    if dir.is_empty() {
        return None;
    }

    Some(PathBuf::from(dir).join(format!("{book_id}-{width}x{height}.jpg")))
}
