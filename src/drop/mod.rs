use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    auth::api_key,
    library::models::{refresh, LibraryState},
};

const PAGE: &str = include_str!("drop.html");
const STYLE: &str = include_str!("oat.min.css");
const MAX_UPLOAD: usize = 512 * 1024 * 1024;

#[derive(Deserialize)]
struct Status {
    status: Option<String>,
}

pub fn create_drop_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route(&format!("{prefix}/drop"), get(page))
        .route(&format!("{prefix}/drop/style.css"), get(style))
        .route(&format!("{prefix}/drop/upload"), post(upload))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
}

async fn page(Query(status): Query<Status>) -> Html<String> {
    Html(render(status.status.as_deref()))
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
            return redirect("name");
        };

        let data = match field.bytes().await {
            Ok(data) if !data.is_empty() => data,
            Ok(_) => return redirect("empty"),
            Err(error) => {
                warn!("upload interrupted while reading {name}: {error}");
                return redirect("read");
            }
        };

        let target = unique(&library_path, &name);

        if let Err(error) = tokio::fs::write(&target, &data).await {
            warn!("cannot write {}: {error}", target.display());

            let status = if error.kind() == std::io::ErrorKind::ReadOnlyFilesystem {
                "readonly"
            } else {
                "write"
            };

            return redirect(status);
        }

        info!("dropped {} ({} bytes)", target.display(), data.len());
        refresh(library).await;

        return redirect("ok");
    }

    redirect("empty")
}

fn redirect(status: &str) -> Response {
    Redirect::to(&format!("/{}/drop?status={status}", api_key())).into_response()
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

fn unique(library_path: &str, name: &str) -> PathBuf {
    let root = Path::new(library_path);
    let candidate = root.join(name);

    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_default();

    for number in 2..1000 {
        let candidate = root.join(format!("{stem} ({number}).{extension}"));

        if !candidate.exists() {
            return candidate;
        }
    }

    candidate
}

fn render(status: Option<&str>) -> String {
    let notice = match status {
        Some("ok") => "<p class=\"notice ok\">Book added. It appears on the reader after the next sync.</p>",
        Some("name") => "<p class=\"notice error\">Only .epub files, please.</p>",
        Some("empty") => "<p class=\"notice error\">No file was attached.</p>",
        Some("read") => "<p class=\"notice error\">The upload was interrupted.</p>",
        Some("write") => "<p class=\"notice error\">The file could not be saved.</p>",
        Some("readonly") => {
            "<p class=\"notice error\">The library folder is mounted read-only. Mount it read-write to accept uploads.</p>"
        }
        _ => "",
    };

    PAGE.replace("{{BASE}}", &format!("/{}", api_key()))
        .replace("{{NOTICE}}", notice)
}
