mod kcc;

use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{multipart::Field, DefaultBodyLimit, Form, Multipart, Query, State},
    http::{header, HeaderMap},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    auth::{api_key, public_base},
    library::models::{refresh, LibraryState},
};

const PAGE: &str = include_str!("drop.html");
const LOGIN: &str = include_str!("login.html");
const STYLE_SHEET: &str = include_str!("oat.min.css");
const PAGE_STYLE: &str = include_str!("page.css");
const APP_JS: &str = include_str!("app.js");
const ICON: &[u8] = include_bytes!("favicon.ico");
const ICON_SMALL: &[u8] = include_bytes!("favicon-32.png");
const ICON_TOUCH: &[u8] = include_bytes!("apple-touch-icon.png");

const MAX_UPLOAD: usize = 512 * 1024 * 1024;
const COOKIE: &str = "kobink";
const COOKIE_AGE: u32 = 30 * 24 * 60 * 60;
const LAZY_ANSWER: Duration = Duration::from_millis(400);
const DEFAULT_JOBS: usize = 2;

#[derive(Deserialize)]
struct Status {
    status: Option<String>,
}

#[derive(Deserialize)]
struct Credentials {
    key: String,
}

enum Kind {
    Book,
    Manga,
}

struct Part {
    name: String,
    path: PathBuf,
    size: u64,
}

pub fn create_drop_router(prefix: &str) -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/", get(root).post(login))
        .route("/upload", post(upload))
        .route("/style.css", get(style))
        .route("/page.css", get(page_style))
        .route("/app.js", get(app_js))
        .route("/favicon.ico", get(icon))
        .route("/favicon-32.png", get(icon_small))
        .route("/apple-touch-icon.png", get(icon_touch))
        .route(&format!("{prefix}/drop"), get(page))
        .route(&format!("{prefix}/drop/upload"), post(page_upload))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
}

async fn root(headers: HeaderMap, Query(status): Query<Status>) -> Response {
    if !authorised(&headers) {
        return Html(login_page(status.status.as_deref())).into_response();
    }

    Html(render("/upload", status.status.as_deref())).into_response()
}

async fn page(Query(status): Query<Status>) -> Html<String> {
    Html(render(
        &format!("/{}/drop/upload", api_key()),
        status.status.as_deref(),
    ))
}

async fn login(Form(credentials): Form<Credentials>) -> Response {
    if !equal(credentials.key.trim(), api_key()) {
        tokio::time::sleep(LAZY_ANSWER).await;
        warn!("drop page: wrong key");

        return Redirect::to("/?status=bad").into_response();
    }

    info!("drop page unlocked");

    let cookie = format!(
        "{COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={COOKIE_AGE}{}",
        session(),
        if public_base().starts_with("https://") {
            "; Secure"
        } else {
            ""
        }
    );

    ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
}

async fn upload(
    State(library): State<Arc<Mutex<LibraryState>>>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Response {
    if !authorised(&headers) {
        return Redirect::to("/").into_response();
    }

    receive(library, multipart, "/").await
}

async fn page_upload(
    State(library): State<Arc<Mutex<LibraryState>>>,
    multipart: Multipart,
) -> Response {
    receive(library, multipart, &format!("/{}/drop", api_key())).await
}

async fn receive(
    library: Arc<Mutex<LibraryState>>,
    mut multipart: Multipart,
    back: &str,
) -> Response {
    let library_path = {
        let library = library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.path.clone()
    };

    let work = work_dir();

    if let Err(error) = tokio::fs::create_dir_all(&work).await {
        warn!("cannot create {}: {error}", work.display());
        return redirect(back, &["write"]);
    }

    let mut parts: Vec<Part> = Vec::new();
    let mut profile = kcc::DEFAULT_PROFILE.to_string();
    let mut interrupted = false;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "profile" => profile = field.text().await.unwrap_or_default(),
            "file" => {
                let file_name = field.file_name().unwrap_or_default().to_string();
                let path = work.join(format!("part-{}", parts.len()));

                match save(field, &path).await {
                    Ok(0) => {
                        let _ = tokio::fs::remove_file(&path).await;
                    }
                    Ok(size) => parts.push(Part {
                        name: file_name,
                        path,
                        size,
                    }),
                    Err(error) => {
                        warn!("upload interrupted while reading {file_name}: {error}");
                        interrupted = true;
                    }
                }
            }
            _ => {}
        }
    }

    if interrupted {
        let _ = tokio::fs::remove_dir_all(&work).await;
        return redirect(back, &["read"]);
    }

    if parts.is_empty() {
        let _ = tokio::fs::remove_dir_all(&work).await;
        return redirect(back, &["empty"]);
    }

    let mut codes = Vec::with_capacity(parts.len());

    for part in parts {
        codes.push(store(&library_path, &work, part, &profile).await);
    }

    let _ = tokio::fs::remove_dir_all(&work).await;
    refresh(library).await;
    redirect(back, &codes)
}

async fn store(library_path: &str, work: &Path, part: Part, profile: &str) -> &'static str {
    let Some(name) = safe_name(&part.name) else {
        return "format";
    };

    match classify(&name) {
        Some(Kind::Book) => {
            let target = unique(library_path, &name);

            match tokio::fs::copy(&part.path, &target).await {
                Ok(_) => {
                    info!("dropped {} ({} bytes)", target.display(), part.size);
                    "ok"
                }
                Err(error) => write_failure(&target, &error),
            }
        }
        Some(Kind::Manga) => {
            if !kcc::available() {
                return "kcc_missing";
            }

            if kcc::profile(profile).is_none() {
                return "profile";
            }

            convert(library_path, work, &part.path, &name, profile).await
        }
        None => "format",
    }
}

async fn convert(
    library_path: &str,
    work: &Path,
    input: &Path,
    name: &str,
    profile: &str,
) -> &'static str {
    let suffix = kcc::extension(profile);
    let title = clean_title(&stem(name));
    let output = work.join(format!("{title}{suffix}"));

    let author = kcc::author(&series(&title), input).await;

    if let Some(author) = author.as_deref() {
        info!("{name}: author {author}");
    }

    let Ok(permit) = jobs().acquire().await else {
        return "write";
    };

    let converted = kcc::convert(input, &output, profile, &title, author.as_deref()).await;
    drop(permit);

    if let Err(error) = converted {
        warn!("{name}: {error}");

        return if error.contains("timed out") {
            "timeout"
        } else {
            "kcc_failed"
        };
    }

    let target = unique(library_path, &format!("{title}{suffix}"));

    match tokio::fs::copy(&output, &target).await {
        Ok(bytes) => {
            info!(
                "converted {name} -> {} ({bytes} bytes, profile {profile})",
                target.display()
            );

            "converted"
        }
        Err(error) => write_failure(&target, &error),
    }
}

fn write_failure(target: &Path, error: &std::io::Error) -> &'static str {
    warn!("cannot write {}: {error}", target.display());

    if error.kind() == std::io::ErrorKind::ReadOnlyFilesystem {
        "readonly"
    } else {
        "write"
    }
}

fn redirect(back: &str, codes: &[&str]) -> Response {
    Redirect::to(&format!("{back}?status={}", codes.join(","))).into_response()
}

fn session() -> &'static str {
    static SESSION: OnceLock<String> = OnceLock::new();

    SESSION.get_or_init(|| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()))
}

fn jobs() -> &'static Semaphore {
    static JOBS: OnceLock<Semaphore> = OnceLock::new();

    JOBS.get_or_init(|| {
        let permits = std::env::var("KCC_JOBS")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_JOBS);

        Semaphore::new(permits)
    })
}

fn authorised(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, value)| name.trim() == COOKIE && equal(value.trim(), session()))
}

fn equal(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());

    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

async fn style() -> Response {
    asset(STYLE_SHEET.as_bytes(), "text/css; charset=utf-8")
}

async fn page_style() -> Response {
    asset(PAGE_STYLE.as_bytes(), "text/css; charset=utf-8")
}

async fn app_js() -> Response {
    asset(APP_JS.as_bytes(), "text/javascript; charset=utf-8")
}

async fn icon() -> Response {
    asset(ICON, "image/x-icon")
}

async fn icon_small() -> Response {
    asset(ICON_SMALL, "image/png")
}

async fn icon_touch() -> Response {
    asset(ICON_TOUCH, "image/png")
}

fn asset(bytes: &'static [u8], content_type: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response()
}

async fn save(mut field: Field<'_>, path: &Path) -> io::Result<u64> {
    let mut file = tokio::fs::File::create(path).await?;
    let mut size = 0u64;

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| io::Error::other(error.to_string()))?
    {
        file.write_all(&chunk).await?;
        size += chunk.len() as u64;
    }

    file.flush().await?;
    Ok(size)
}

fn work_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("kobink-{}-{stamp}", std::process::id()))
}

fn safe_name(name: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return None;
    }

    Some(Path::new(name).file_name()?.to_string_lossy().to_string())
}

fn classify(name: &str) -> Option<Kind> {
    let name = name.to_lowercase();

    if name.ends_with(".epub") {
        Some(Kind::Book)
    } else if name.ends_with(".cbz") || name.ends_with(".pdf") {
        Some(Kind::Manga)
    } else {
        None
    }
}

fn stem(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string())
}

fn clean_title(title: &str) -> String {
    let mut cleaned = String::with_capacity(title.len());
    let mut depth = 0usize;

    for character in title.chars() {
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '_' if depth == 0 => cleaned.push(' '),
            _ if depth == 0 => cleaned.push(character),
            _ => {}
        }
    }

    let cleaned = cleaned.trim_end_matches(|c: char| c == '.' || c.is_whitespace());

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn series(title: &str) -> String {
    let tokens: Vec<&str> = title.split_whitespace().collect();

    for (index, token) in tokens.iter().enumerate() {
        if index == 0 {
            continue;
        }

        let lowered = token.to_lowercase();
        let bare = lowered.trim_end_matches('.');
        let number = bare
            .strip_prefix('#')
            .or_else(|| bare.strip_prefix("chapter"))
            .or_else(|| bare.strip_prefix("volume"))
            .or_else(|| bare.strip_prefix("vol"))
            .or_else(|| bare.strip_prefix('v'));

        let next_is_number = tokens
            .get(index + 1)
            .is_some_and(|next| next.chars().all(|c| c.is_ascii_digit()));

        let is_marker = match number {
            Some(rest) => {
                (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
                    || (rest.is_empty() && next_is_number)
            }
            None => false,
        };

        if is_marker {
            return tokens[..index].join(" ");
        }
    }

    title.to_string()
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

fn controls() -> String {
    if !kcc::available() {
        return "<p class=\"hint\">CBZ and PDF need an image built with KCC. EPUB works either way.</p>"
            .to_string();
    }

    let mut groups: Vec<(&str, String)> = Vec::new();

    for (code, label, group) in kcc::PROFILES {
        let selected = if *code == kcc::DEFAULT_PROFILE {
            " selected"
        } else {
            ""
        };
        let option = format!("<option value=\"{code}\"{selected}>{label}</option>");

        match groups.iter_mut().position(|entry| entry.0 == *group) {
            Some(index) => groups[index].1.push_str(&option),
            None => groups.push((*group, option)),
        }
    }

    let mut html = String::from(
        "<div class=\"field\"><span>E-reader for CBZ and PDF</span><select name=\"profile\">",
    );

    for (group, options) in groups {
        html.push_str(&format!("<optgroup label=\"{group}\">{options}</optgroup>"));
    }

    html.push_str("</select></div>");
    html
}

fn message(code: &str) -> Option<(&'static str, &'static str)> {
    match code {
        "ok" => Some(("ok", "Book added. It appears on the reader after the next sync.")),
        "converted" => Some((
            "ok",
            "Converted and added. It appears on the reader after the next sync.",
        )),
        "format" => Some(("error", "Only .epub, .cbz and .pdf, please.")),
        "profile" => Some(("error", "Unknown e-reader.")),
        "empty" => Some(("error", "No file was attached.")),
        "read" => Some(("error", "The upload was interrupted.")),
        "write" => Some(("error", "A file could not be saved.")),
        "kcc_missing" => Some((
            "error",
            "This image has no KCC, so CBZ and PDF cannot be converted. EPUB still works.",
        )),
        "kcc_failed" => Some((
            "error",
            "KCC could not convert a file. The container log has the reason.",
        )),
        "timeout" => Some(("error", "A conversion took too long and was stopped.")),
        "readonly" => Some((
            "error",
            "The library folder is mounted read-only. Mount it read-write to accept uploads.",
        )),
        _ => None,
    }
}

fn notices(status: Option<&str>) -> String {
    status
        .unwrap_or_default()
        .split(',')
        .filter_map(|code| message(code.trim()))
        .map(|(tone, text)| format!("<p class=\"notice {tone}\">{text}</p>"))
        .collect()
}

fn render(action: &str, status: Option<&str>) -> String {
    PAGE.replace("{{ACTION}}", action)
        .replace("{{CONTROLS}}", &controls())
        .replace("{{NOTICE}}", &notices(status))
}

fn login_page(status: Option<&str>) -> String {
    let notice = match status {
        Some("bad") => "<p class=\"notice\">That key does not match.</p>",
        _ => "",
    };

    LOGIN.replace("{{NOTICE}}", notice)
}
