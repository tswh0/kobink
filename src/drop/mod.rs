mod kcc;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::header,
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

    let mut file: Option<(String, Bytes)> = None;
    let mut manga = false;
    let mut profile = kcc::DEFAULT_PROFILE.to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "mode" => manga = matches!(field.text().await.as_deref(), Ok("manga")),
            "profile" => profile = field.text().await.unwrap_or_default(),
            "file" => {
                let file_name = field.file_name().unwrap_or_default().to_string();
                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(error) => {
                        warn!("upload interrupted while reading: {error}");
                        return redirect("read");
                    }
                };
                file = Some((file_name, data));
            }
            _ => {}
        }
    }

    let Some((raw_name, data)) = file else {
        return redirect("empty");
    };

    if data.is_empty() {
        return redirect("empty");
    }

    if manga {
        return convert_upload(library, &library_path, raw_name, data, profile).await;
    }

    drop_book(library, &library_path, raw_name, data).await
}

async fn drop_book(
    library: Arc<Mutex<LibraryState>>,
    library_path: &str,
    raw_name: String,
    data: Bytes,
) -> Response {
    let Some(name) = file_name(&raw_name, ".epub") else {
        return redirect("book");
    };

    let target = unique(library_path, &name);

    if let Err(error) = tokio::fs::write(&target, &data).await {
        return write_failure(&target, &error);
    }

    info!("dropped {} ({} bytes)", target.display(), data.len());
    refresh(library).await;

    redirect("ok")
}

async fn convert_upload(
    library: Arc<Mutex<LibraryState>>,
    library_path: &str,
    raw_name: String,
    data: Bytes,
    profile: String,
) -> Response {
    if !kcc::available() {
        return redirect("kcc_missing");
    }

    let Some(name) = file_name(&raw_name, ".cbz") else {
        return redirect("manga");
    };

    if kcc::profile(&profile).is_none() {
        return redirect("profile");
    }

    let suffix = kcc::extension(&profile);
    let work = work_dir();
    let input = work.join(&name);
    let title = clean_title(&stem(&name));
    let output = work.join(format!("{title}{suffix}"));

    if let Err(error) = tokio::fs::create_dir_all(&work).await {
        warn!("cannot create {}: {error}", work.display());
        return redirect("write");
    }

    if let Err(error) = tokio::fs::write(&input, &data).await {
        warn!("cannot write {}: {error}", input.display());
        return write_failure(&input, &error);
    }

    let author = kcc::author(&series(&title), &input).await;

    if let Some(author) = author.as_deref() {
        info!("{name}: author {author}");
    }

    let converted = kcc::convert(&input, &output, &profile, &title, author.as_deref()).await;
    let _ = tokio::fs::remove_file(&input).await;

    if let Err(error) = converted {
        warn!("{name}: {error}");
        let _ = tokio::fs::remove_dir_all(&work).await;

        return redirect(if error.contains("timed out") {
            "timeout"
        } else {
            "kcc_failed"
        });
    }

    let target = unique(library_path, &format!("{title}{suffix}"));

    if let Err(error) = tokio::fs::copy(&output, &target).await {
        let _ = tokio::fs::remove_dir_all(&work).await;
        return write_failure(&target, &error);
    }

    let _ = tokio::fs::remove_dir_all(&work).await;

    info!(
        "converted {} -> {} ({} bytes in, profile {profile})",
        name,
        target.display(),
        data.len()
    );

    refresh(library).await;
    redirect("converted")
}

fn write_failure(target: &Path, error: &std::io::Error) -> Response {
    warn!("cannot write {}: {error}", target.display());

    redirect(if error.kind() == std::io::ErrorKind::ReadOnlyFilesystem {
        "readonly"
    } else {
        "write"
    })
}

fn redirect(status: &str) -> Response {
    Redirect::to(&format!("/{}/drop?status={status}", api_key())).into_response()
}

fn work_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("kobink-{}-{stamp}", std::process::id()))
}

fn file_name(name: &str, extension: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return None;
    }

    let name = Path::new(name).file_name()?.to_string_lossy().to_string();

    if !name.to_lowercase().ends_with(extension) {
        return None;
    }

    Some(name)
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
            Some(rest) => (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())) || (rest.is_empty() && next_is_number),
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
        return "<p class=\"hint\">Manga conversion needs an image built with KCC.</p>".to_string();
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
        "<div class=\"field\"><span>What is it?</span>\
         <select name=\"mode\">\
         <option value=\"book\">A book, add it as it is</option>\
         <option value=\"manga\">Manga, convert the CBZ</option>\
         </select></div>\
         <div class=\"field\"><span>E-reader</span><select name=\"profile\">",
    );

    for (group, options) in groups {
        html.push_str(&format!("<optgroup label=\"{group}\">{options}</optgroup>"));
    }

    html.push_str("</select></div>");
    html
}

fn render(status: Option<&str>) -> String {
    let notice = match status {
        Some("ok") => "<p class=\"notice ok\">Book added. It appears on the reader after the next sync.</p>",
        Some("converted") => {
            "<p class=\"notice ok\">Converted and added. It appears on the reader after the next sync.</p>"
        }
        Some("book") => "<p class=\"notice error\">Only .epub files, please.</p>",
        Some("manga") => "<p class=\"notice error\">Manga conversion takes a .cbz file.</p>",
        Some("profile") => "<p class=\"notice error\">Unknown e-reader.</p>",
        Some("empty") => "<p class=\"notice error\">No file was attached.</p>",
        Some("read") => "<p class=\"notice error\">The upload was interrupted.</p>",
        Some("write") => "<p class=\"notice error\">The file could not be saved.</p>",
        Some("kcc_missing") => {
            "<p class=\"notice error\">This image has no KCC, so manga cannot be converted.</p>"
        }
        Some("kcc_failed") => {
            "<p class=\"notice error\">KCC could not convert that file. The container log has the reason.</p>"
        }
        Some("timeout") => "<p class=\"notice error\">Conversion took too long and was stopped.</p>",
        Some("readonly") => {
            "<p class=\"notice error\">The library folder is mounted read-only. Mount it read-write to accept uploads.</p>"
        }
        _ => "",
    };

    PAGE.replace("{{BASE}}", &format!("/{}", api_key()))
        .replace("{{CONTROLS}}", &controls())
        .replace("{{NOTICE}}", notice)
}
