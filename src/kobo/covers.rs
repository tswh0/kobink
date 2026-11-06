use std::{
    fs,
    io::Cursor,
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
use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ExtendedColorType, ImageReader,
    Limits,
};
use tracing::warn;

use crate::library::models::LibraryState;

const DEFAULT_DIMENSION: u32 = 600;
const MAX_DIMENSION: u32 = 3000;
const JPEG_QUALITY: u8 = 80;
const PREWARM: [(u32, u32); 3] = [(149, 223), (355, 530), (1264, 1680)];
const MAX_COVER_BYTES: usize = 24 * 1024 * 1024;
const MAX_COVER_SIDE: u32 = 12_000;
const MAX_COVER_ALLOC: u64 = 256 * 1024 * 1024;

pub fn create_cover_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new().route(
        &format!("{prefix}/{{book_id}}/{{width}}/{{height}}/{{*image}}"),
        get(handle_cover),
    )
}

async fn handle_cover(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path((book_id, width, height, _)): Path<(String, String, String, String)>,
) -> Response {
    let path = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    if let Some(bytes) = cache_path(book_id, width, height).and_then(|path| fs::read(path).ok()) {
        return Some(bytes);
    }

    let mut doc = EpubDoc::new(&path).ok()?;
    let (data, _) = doc.get_cover()?;

    if data.len() > MAX_COVER_BYTES {
        warn!(
            "cover of {} is {} bytes, over the {} byte limit",
            path.display(),
            data.len(),
            MAX_COVER_BYTES
        );
        return None;
    }

    let Some(decoded) = decode(&data) else {
        warn!("cannot decode the cover of {}", path.display());
        return None;
    };

    let requested = render(&decoded, width, height)?;
    store(book_id, width, height, &requested);

    for (warm_width, warm_height) in PREWARM {
        if (warm_width, warm_height) == (width, height) {
            continue;
        }
        if cache_path(book_id, warm_width, warm_height).is_some_and(|path| path.exists()) {
            continue;
        }
        if let Some(bytes) = render(&decoded, warm_width, warm_height) {
            store(book_id, warm_width, warm_height, &bytes);
        }
    }

    Some(requested)
}

fn decode(data: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()?;

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_COVER_SIDE);
    limits.max_image_height = Some(MAX_COVER_SIDE);
    limits.max_alloc = Some(MAX_COVER_ALLOC);
    reader.limits(limits);

    reader.decode().ok()
}

fn render(image: &DynamicImage, width: u32, height: u32) -> Option<Vec<u8>> {
    let fitted = if image.width() <= width && image.height() <= height {
        image.clone()
    } else {
        image.resize(width, height, FilterType::Triangle)
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

    Some(bytes)
}

fn store(book_id: &str, width: u32, height: u32, bytes: &[u8]) {
    if let Some(path) = cache_path(book_id, width, height) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, bytes);
    }
}

fn cache_path(book_id: &str, width: u32, height: u32) -> Option<PathBuf> {
    let dir = std::env::var("COVER_CACHE_DIR").unwrap_or_else(|_| "/app/data/covers".to_string());

    if dir.is_empty() {
        return None;
    }

    Some(PathBuf::from(dir).join(format!("{book_id}-{width}x{height}.jpg")))
}
