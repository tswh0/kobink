use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as UrlPath, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use tracing::{info, warn};

use crate::{
    auth::api_key,
    library::models::{refresh, Book, LibraryState},
};

const DASHBOARD: &str = include_str!("dashboard.html");
const STYLE: &str = include_str!("oat.min.css");
const MAX_UPLOAD: usize = 512 * 1024 * 1024;

pub fn create_admin_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route(&format!("{prefix}/admin"), get(dashboard))
        .route(&format!("{prefix}/admin/style.css"), get(style))
        .route(&format!("{prefix}/admin/upload"), post(upload))
        .route(&format!("{prefix}/admin/rescan"), post(rescan))
        .route(&format!("{prefix}/admin/delete/{{book_id}}"), post(delete))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
}

async fn dashboard(State(library): State<Arc<Mutex<LibraryState>>>) -> Response {
    let (books, path) = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (library.books.clone(), library.path.clone())
    };

    (
        [(header::CACHE_CONTROL, "no-store")],
        Html(render(&books, &path)),
    )
        .into_response()
}

async fn style() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        STYLE,
    )
        .into_response()
}

async fn rescan(State(library): State<Arc<Mutex<LibraryState>>>) -> Response {
    refresh(library).await;
    redirect()
}

async fn upload(
    State(library): State<Arc<Mutex<LibraryState>>>,
    mut multipart: Multipart,
) -> Response {
    let library_path = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.path.clone()
    };

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }

        let Some(name) = file_name(field.file_name().unwrap_or_default()) else {
            return (StatusCode::BAD_REQUEST, "only .epub files, please").into_response();
        };

        let data = match field.bytes().await {
            Ok(data) => data,
            Err(error) => {
                warn!("upload failed while reading {name}: {error}");
                return (StatusCode::BAD_REQUEST, "upload interrupted").into_response();
            }
        };

        let target = PathBuf::from(&library_path).join(&name);
        if let Err(error) = tokio::fs::write(&target, &data).await {
            warn!("cannot write {}: {error}", target.display());
            return (StatusCode::INTERNAL_SERVER_ERROR, "cannot store the file").into_response();
        }

        info!("uploaded {} ({} bytes)", target.display(), data.len());
        refresh(library).await;

        return redirect();
    }

    (StatusCode::BAD_REQUEST, "no file in the upload").into_response()
}

async fn delete(
    State(library): State<Arc<Mutex<LibraryState>>>,
    UrlPath(book_id): UrlPath<String>,
) -> Response {
    let (book, library_path) = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (
            library.find_book_by_id(&book_id).map(|book| book.path.clone()),
            library.path.clone(),
        )
    };

    let Some(path) = book else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if !inside(&path, &library_path) {
        warn!("refusing to delete {} outside the library", path.display());
        return StatusCode::FORBIDDEN.into_response();
    }

    if let Err(error) = tokio::fs::remove_file(&path).await {
        warn!("cannot delete {}: {error}", path.display());
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    info!("deleted {}", path.display());
    refresh(library).await;

    redirect()
}

fn redirect() -> Response {
    Redirect::to(&format!("/{}/admin", api_key())).into_response()
}

fn file_name(name: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return None;
    }

    let name = Path::new(name).file_name()?.to_string_lossy().to_string();

    if !name.to_lowercase().ends_with(".epub") {
        return None;
    }

    Some(name)
}

fn inside(path: &Path, library_path: &str) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    let Ok(root) = Path::new(library_path).canonicalize() else {
        return false;
    };

    path.starts_with(root)
}

fn render(books: &[Book], library_path: &str) -> String {
    let base = format!("/{}", api_key());
    let rows = if books.is_empty() {
        "<tr><td colspan=\"6\">No books yet. Upload one above.</td></tr>".to_string()
    } else {
        books
            .iter()
            .map(|book| row(book, &base))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let total: u64 = books.iter().map(|book| book.size).sum();

    DASHBOARD
        .replace("{{BASE}}", &base)
        .replace("{{COUNT}}", &books.len().to_string())
        .replace("{{SIZE}}", &megabytes(total))
        .replace("{{PATH}}", &escape(library_path))
        .replace("{{ROWS}}", &rows)
}

fn row(book: &Book, base: &str) -> String {
    let series = match &book.series {
        Some(series) => format!(
            "{} #{}",
            escape(&series.name),
            series.number.round() as u64
        ),
        None => String::from("&mdash;"),
    };

    format!(
        "<tr>\
<td><img src=\"{base}/{id}/74/112/false/image.jpg\" width=\"37\" height=\"56\" alt=\"\" loading=\"lazy\"></td>\
<td>{title}</td>\
<td>{author}</td>\
<td>{series}</td>\
<td>{size}</td>\
<td><form method=\"post\" action=\"{base}/admin/delete/{id}\">\
<button class=\"small\" data-variant=\"danger\">Delete</button></form></td>\
</tr>",
        base = base,
        id = book.id,
        title = escape(&book.title),
        author = escape(&book.author),
        series = series,
        size = megabytes(book.size),
    )
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }

    escaped
}
