use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::info;

use crate::auth::api_key;

const BODY_LIMIT: usize = 8192;
const LOGGED_PATH: &str = "/kobo/v1/";

fn scrub(path: &str) -> String {
    path.replacen(&format!("/{}", api_key()), "/<key>", 1)
}

pub async fn log_request_response(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let (parts, body) = next.run(req).await.into_parts();
    let path = scrub(&path);
    info!("{method} {path} -> {}", parts.status);

    let length = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());

    if !path.contains(LOGGED_PATH) || !length.is_some_and(|length| length <= BODY_LIMIT) {
        return Response::from_parts(parts, body);
    }

    match to_bytes(body, BODY_LIMIT).await {
        Ok(bytes) if bytes.is_empty() => {
            info!("response {path}: <empty>");
            Response::from_parts(parts, Body::from(bytes))
        }
        Ok(bytes) => {
            info!("response {path}: {}", String::from_utf8_lossy(&bytes));
            Response::from_parts(parts, Body::from(bytes))
        }
        Err(error) => {
            info!("response {path}: unreadable body ({error})");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
