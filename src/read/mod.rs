use std::{
    cmp::Ordering,
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::{
    body::Body,
    extract::{Path as Route, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, put},
    Json, Router,
};
use epub::doc::{EpubDoc, NavPoint};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    auth::api_key,
    drop::{authorised, Status},
    library::{
        epub::{decode, first_image, resolve},
        models::{Book, LibraryState},
        progress::{self, Progress},
    },
};

const SHELF: &str = include_str!("shelf.html");
const OFFLINE: &str = include_str!("offline.html");
const MANIFEST: &str = include_str!("manifest.webmanifest");
const WORKER: &str = include_str!("sw.js");
const PWA: &str = include_str!("pwa.js");
const SHELF_JS: &str = include_str!("shelf.js");
const ICON_192: &[u8] = include_bytes!("icon-192.png");
const ICON_512: &[u8] = include_bytes!("icon-512.png");
const ICON_MASKABLE: &[u8] = include_bytes!("icon-maskable-512.png");
const READER: &str = include_str!("reader.html");
const STYLE: &str = include_str!("reader.css");
const SCRIPT: &str = include_str!("reader.js");

const COVER: (u32, u32) = (149, 223);
const MAX_CACHED_BOOKS: usize = 3;
const MAX_CACHED_BYTES: u64 = 96 * 1024 * 1024;

struct CachedBook {
    path: PathBuf,
    size: u64,
    bytes: Arc<Vec<u8>>,
}

static BOOKS: Mutex<Vec<CachedBook>> = Mutex::new(Vec::new());

pub fn create_read_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/", get(shelf))
        .route("/read", get(move || async { Redirect::to("/") }))
        .route("/offline", get(offline))
        .route("/manifest.webmanifest", get(manifest_json))
        .route("/sw.js", get(worker))
        .route("/pwa.js", get(pwa))
        .route("/shelf.js", get(shelf_script))
        .route("/icon-192.png", get(icon_192))
        .route("/icon-512.png", get(icon_512))
        .route("/icon-maskable-512.png", get(icon_maskable))
        .route("/reader.css", get(style))
        .route("/reader.js", get(script))
        .route("/read/{book_id}", get(reader))
        .route("/read/{book_id}/manifest", get(manifest))
        .route("/read/{book_id}/progress", put(write_progress))
        .route("/read/{book_id}/res/{*path}", get(resource))
}

#[derive(Debug, Serialize)]
struct Manifest {
    title: String,
    author: String,
    layout: &'static str,
    chapters: Vec<Chapter>,
    pages: Vec<Page>,
    progress: View,
}

#[derive(Debug, Serialize, Clone)]
struct Chapter {
    href: String,
    label: String,
}

#[derive(Debug, Serialize)]
struct Page {
    href: String,
    src: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct View {
    percent: f64,
    chapter: String,
    chapter_percent: f64,
    page: u32,
    status: String,
}

impl From<Option<Progress>> for View {
    fn from(progress: Option<Progress>) -> Self {
        progress
            .map(|progress| Self {
                percent: progress.percent,
                chapter: progress.chapter,
                chapter_percent: progress.chapter_percent,
                page: progress.page,
                status: progress.status,
            })
            .unwrap_or_default()
    }
}

async fn shelf(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    Query(status): Query<Status>,
) -> Response {
    if !authorised(&headers) {
        return Html(crate::drop::login_page(status.status.as_deref())).into_response();
    }

    let (library_path, books) = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (state.path.clone(), state.books.clone())
    };

    let states = tokio::task::spawn_blocking(move || progress::all(&library_path))
        .await
        .unwrap_or_default();

    let active = |state: Option<&Progress>| {
        state.is_some_and(|progress| {
            progress.percent > 0.0
                && progress.status != "Finished"
                && progress.status != "ReadyToRead"
        })
    };

    let reading = states.values().filter(|progress| active(Some(progress))).count();

    let mut order: Vec<&Book> = books.iter().collect();
    order.sort_by(|left, right| {
        let left_state = states.get(&left.id.to_string());
        let right_state = states.get(&right.id.to_string());

        match (active(left_state), active(right_state)) {
            (true, true) => {
                let left_stamp = left_state.map(|state| state.updated).unwrap_or_default();
                let right_stamp = right_state.map(|state| state.updated).unwrap_or_default();

                right_stamp
                    .cmp(&left_stamp)
                    .then_with(|| left.title.cmp(&right.title))
            }
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => left.title.cmp(&right.title),
        }
    });

    let mut markup = String::new();

    for book in order {
        let state = states.get(&book.id.to_string());
        let percent = state.map(|state| state.percent).unwrap_or_default();
        let cover = format!(
            "/{}/{}/{}/{}/false/image.jpg",
            api_key(),
            book.id,
            COVER.0,
            COVER.1
        );

        let mut meter = String::new();

        if percent > 0.0 {
            let label = if percent >= 99.5 {
                "Finished".to_string()
            } else {
                format!("{:.0}%", percent)
            };

            meter = format!(
                r#"<span class="read-line"><progress max="100" value="{percent:.1}"></progress><span>{label}</span></span>"#
            );
        }

        markup.push_str(&format!(
            r#"<li><a class="book" href="/read/{id}"><span class="cover" data-initials="{initials}"><img src="{cover}" alt="" loading="lazy"></span><span class="name">{title}</span><span class="by">{author}</span>{meter}</a></li>"#,
            id = book.id,
            initials = escape(&initials(&book.title)),
            cover = cover,
            title = escape(&book.title),
            author = escape(&book.author),
            meter = meter,
        ));
        markup.push('\n');
    }

    let lede = if reading > 0 {
        format!("{} books · {reading} in progress", books.len())
    } else {
        format!("{} books", books.len())
    };

    Html(SHELF.replace("{{COUNT}}", &lede).replace("{{BOOKS}}", &markup)).into_response()
}

async fn reader(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    Route(book_id): Route<String>,
) -> Response {
    if !authorised(&headers) {
        return Redirect::to("/").into_response();
    }

    let book = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        state.find_book_by_id(&book_id).cloned()
    };

    let Some(book) = book else {
        return StatusCode::NOT_FOUND.into_response();
    };

    Html(
        READER
            .replace("{{TITLE}}", &escape(&book.title))
            .replace("{{ID}}", &book.id.to_string()),
    )
    .into_response()
}

async fn manifest(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    Route(book_id): Route<String>,
) -> Response {
    if !authorised(&headers) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let (book, library_path) = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        (state.find_book_by_id(&book_id).cloned(), state.path.clone())
    };

    let Some(book) = book else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let path = book.path.clone();
    let fixed = book.fixed_layout;
    let structure = match tokio::task::spawn_blocking(move || structure_of(&path, fixed)).await {
        Ok(Some(structure)) => structure,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            warn!("cannot read {}: {error}", book.path.display());
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let store = library_path.clone();
    let id = book_id.clone();
    let view = tokio::task::spawn_blocking(move || View::from(progress::load(&store, &id)))
        .await
        .unwrap_or_default();

    Json(Manifest {
        title: book.title.clone(),
        author: book.author.clone(),
        layout: if fixed { "fixed" } else { "flow" },
        chapters: structure.chapters,
        pages: structure.pages,
        progress: view,
    })
    .into_response()
}

async fn write_progress(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    Route(book_id): Route<String>,
    Json(view): Json<View>,
) -> Response {
    if !authorised(&headers) {
        return StatusCode::NOT_FOUND.into_response();
    }

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

    let wanted_id = book_id.clone();
    let stored = tokio::task::spawn_blocking(move || {
        let incoming = Progress {
            percent: view.percent.clamp(0.0, 100.0),
            chapter: view.chapter,
            chapter_percent: view.chapter_percent.clamp(0.0, 100.0),
            page: view.page,
            status: if view.status.is_empty() {
                "Reading".to_string()
            } else {
                view.status
            },
            ..Default::default()
        };

        let mut merged = progress::merge(progress::load(&library_path, &wanted_id), incoming);

        progress::save(&library_path, &wanted_id, &mut merged)
    })
    .await;

    match stored {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(error)) => {
            warn!("cannot store the progress of {book_id}: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(error) => {
            warn!("cannot store the progress of {book_id}: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn resource(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    Route((book_id, path)): Route<(String, String)>,
) -> Response {
    if !authorised(&headers) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let book = {
        let state = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        state.find_book_by_id(&book_id).cloned()
    };

    let Some(book) = book else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let wanted = decode(path.trim_start_matches('/'));

    if wanted.is_empty() || wanted.contains("..") || wanted.starts_with('/') {
        return StatusCode::NOT_FOUND.into_response();
    }

    let served = book.path.clone();
    let opened = tokio::task::spawn_blocking(move || open_resource(&served, &wanted)).await;

    match opened {
        Ok(Some((bytes, mime))) => {
            let mut response = Response::new(Body::from(bytes));
            let mime = HeaderValue::try_from(mime).unwrap_or(HeaderValue::from_static(OCTET));
            response.headers_mut().insert(header::CONTENT_TYPE, mime);
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=600"),
            );

            response
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            warn!("cannot serve a file of {}: {error}", book.path.display());
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

const OCTET: &str = "application/octet-stream";

async fn offline() -> Response {
    asset(OFFLINE.as_bytes(), "text/html; charset=utf-8")
}

async fn manifest_json() -> Response {
    asset(MANIFEST.as_bytes(), "application/manifest+json")
}

async fn worker() -> Response {
    asset(WORKER.as_bytes(), "text/javascript; charset=utf-8")
}

async fn pwa() -> Response {
    asset(PWA.as_bytes(), "text/javascript; charset=utf-8")
}

async fn shelf_script() -> Response {
    asset(SHELF_JS.as_bytes(), "text/javascript; charset=utf-8")
}

async fn icon_192() -> Response {
    icon(ICON_192)
}

async fn icon_512() -> Response {
    icon(ICON_512)
}

async fn icon_maskable() -> Response {
    icon(ICON_MASKABLE)
}

fn icon(bytes: &'static [u8]) -> Response {
    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );

    response
}

async fn style() -> Response {
    asset(STYLE.as_bytes(), "text/css; charset=utf-8")
}

async fn script() -> Response {
    asset(SCRIPT.as_bytes(), "text/javascript; charset=utf-8")
}

fn asset(bytes: &'static [u8], mime: &'static str) -> Response {
    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    response
}

struct Structure {
    chapters: Vec<Chapter>,
    pages: Vec<Page>,
}

fn structure_of(path: &Path, fixed: bool) -> Option<Structure> {
    let mut doc = EpubDoc::new(path).ok()?;
    let mut labels: HashMap<String, String> = HashMap::new();
    index_toc(&doc.toc, &mut labels);

    if labels.is_empty() {
        if let Some((nav, html)) = nav_document(&mut doc) {
            for (href, label) in nav_points(&html) {
                labels.entry(resolve(&nav, &href)).or_insert(label);
            }
        }
    }

    let mut chapters = Vec::new();

    for item in &doc.spine {
        if !item.linear {
            continue;
        }

        let Some(resource) = doc.resources.get(&item.idref) else {
            continue;
        };
        let href = resource.path.to_string_lossy().replace('\\', "/");
        let label = labels.get(&href).cloned().unwrap_or_default();

        chapters.push(Chapter { href, label });
    }

    let mut pages = Vec::new();

    if fixed {
        for chapter in &chapters {
            let Some(page) = doc.get_resource_str_by_path(&chapter.href) else {
                continue;
            };
            let Some(src) = first_image(&page) else {
                continue;
            };

            pages.push(Page {
                href: chapter.href.clone(),
                src: resolve(&chapter.href, &src),
            });
        }
    }

    Some(Structure { chapters, pages })
}

fn index_toc(points: &[NavPoint], labels: &mut HashMap<String, String>) {
    for point in points {
        let href = point.content.to_string_lossy().replace('\\', "/");
        let href = decode(href.split('#').next().unwrap_or_default());
        let label = point.label.trim();

        if !label.is_empty() {
            labels.entry(href).or_insert_with(|| label.to_string());
        }

        index_toc(&point.children, labels);
    }
}

fn nav_document<R: std::io::Read + std::io::Seek>(doc: &mut EpubDoc<R>) -> Option<(String, String)> {
    let path = doc
        .resources
        .values()
        .find(|resource| {
            resource
                .properties
                .as_deref()
                .is_some_and(|properties| properties.split_whitespace().any(|token| token == "nav"))
        })
        .or_else(|| {
            doc.resources.values().find(|resource| {
                let name = resource.path.to_string_lossy();

                name.ends_with("nav.xhtml") || name.ends_with("toc.xhtml")
            })
        })
        .map(|resource| resource.path.to_string_lossy().replace('\\', "/"))?;

    let html = doc.get_resource_str_by_path(&path)?;

    Some((path, html))
}

fn nav_points(html: &str) -> Vec<(String, String)> {
    let mut points = Vec::new();
    let mut rest = html;

    while let Some(at) = rest.find("<a ") {
        rest = &rest[at..];

        let Some(href_at) = rest.find("href=") else {
            break;
        };
        let quote = rest[href_at + 5..].chars().next().unwrap_or('"');
        let start = href_at + 6;

        let (Some(end), Some(text_at)) = (rest[start..].find(quote), rest.find('>')) else {
            break;
        };

        let href = decode(&rest[start..start + end]);
        let Some(close) = rest[text_at..].find("</a>") else {
            break;
        };
        let label = strip_tags(&rest[text_at + 1..text_at + close]);

        if !href.is_empty() {
            points.push((href, label));
        }

        rest = &rest[text_at + close + 4..];
    }

    points
}

fn strip_tags(html: &str) -> String {
    let mut text = String::new();
    let mut inside = false;

    for character in html.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            other if !inside => text.push(other),
            _ => {}
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn initials(title: &str) -> String {
    title
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .take(2)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

fn escape(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn book_bytes(path: &Path) -> Option<Arc<Vec<u8>>> {
    let meta = std::fs::metadata(path).ok()?;
    let mut cache = BOOKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(at) = cache
        .iter()
        .position(|cached| cached.path == path && cached.size == meta.len())
    {
        let entry = cache.remove(at);
        let bytes = entry.bytes.clone();
        drop(cache);

        return Some(bytes);
    }

    drop(cache);

    if meta.len() > MAX_CACHED_BYTES {
        return std::fs::read(path).ok().map(Arc::new);
    }

    let bytes = Arc::new(std::fs::read(path).ok()?);
    let mut cache = BOOKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    while cache.len() >= MAX_CACHED_BOOKS {
        cache.remove(0);
    }

    cache.push(CachedBook {
        path: path.to_path_buf(),
        size: meta.len(),
        bytes: bytes.clone(),
    });

    Some(bytes)
}

fn open_resource(path: &Path, wanted: &str) -> Option<(Vec<u8>, String)> {
    let bytes = book_bytes(path)?;
    let mut doc = EpubDoc::from_reader(Cursor::new(&bytes[..])).ok()?;

    let resource = doc.resources.values().find(|resource| {
        let path = resource.path.to_string_lossy().replace('\\', "/");

        path == wanted || decode(&path) == wanted
    })?;
    let mime = resource.mime.clone();
    let path = resource.path.clone();
    let content = doc
        .get_resource_by_path(&path)
        .or_else(|| doc.get_resource_by_path(decode(&path.to_string_lossy())))?;

    Some((content, mime))
}
