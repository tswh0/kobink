use std::sync::{Arc, Mutex, OnceLock};

use crate::{
    auth::{api_key, public_base},
    library::models::LibraryState,
};

use super::{
    library::create_library_router,
    products,
    models::{AuthDeviceRequest, AuthDeviceResponse},
};

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

const INITIALIZATION_DATA: &str = include_str!("./data/initialization.json");

pub fn create_kobo_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/auth/device", post(handle_auth_device))
        .route("/user/profile", get(handle_profile))
        .route("/user/loyalty/benefits", get(stub))
        .route("/user/recommendations", get(handle_recommendations))
        .route("/deals", get(handle_deals))
        .route("/products/featured", get(handle_featured))
        .route("/products/featured/", get(handle_featured))
        .route("/products/{book_id}/nextread", get(handle_nextread))
        .route("/products/{book_id}/prices", get(stub))
        .route("/products/books/series/{series_id}", get(stub))
        .route("/analytics/{*_}", get(stub).post(stub))
        .route("/affiliate", get(stub))
        .route("/initialization", get(handle_initialization))
        .route("/user/wishlist", get(stub))
        .nest("/library", create_library_router())
}

async fn handle_auth_device(Json(_): Json<AuthDeviceRequest>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(AuthDeviceResponse::sample()),
    )
}

async fn handle_initialization() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert("x-kobo-apitoken", HeaderValue::from_static("e30="));

    (headers, initialization()).into_response()
}

fn initialization() -> &'static str {
    static INITIALIZED: OnceLock<String> = OnceLock::new();

    INITIALIZED.get_or_init(|| {
        let mut root: Value =
            serde_json::from_str(INITIALIZATION_DATA).unwrap_or(Value::Null);
        let base = public_base();
        let key = api_key();

        if let Some(resources) = root.get_mut("Resources").and_then(Value::as_object_mut) {
            resources.insert("image_host".to_string(), Value::String(base.to_string()));
            resources.insert(
                "image_url_template".to_string(),
                Value::String(format!(
                    "{base}/{key}/{{ImageId}}/{{width}}/{{height}}/false/image.jpg"
                )),
            );
            resources.insert(
                "image_url_quality_template".to_string(),
                Value::String(format!(
                    "{base}/{key}/{{ImageId}}/{{width}}/{{height}}/{{Quality}}/{{isGreyscale}}/image.jpg"
                )),
            );
            resources.insert(
                "library_sync".to_string(),
                Value::String(format!("{base}/{key}/kobo/v1/library/sync")),
            );
        }

        root.to_string()
    })
}

const SHELF: usize = 6;

fn disabled() -> bool {
    std::env::var("KOBO_STORE").as_deref() == Ok("off")
}

fn snapshot(library: &Arc<Mutex<LibraryState>>) -> Arc<Vec<crate::library::models::Book>> {
    library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .books
        .clone()
}

async fn handle_profile() -> Json<Value> {
    if disabled() {
        return Json(json!({}));
    }

    let name = std::env::var("KOBO_NAME").unwrap_or_else(|_| "kobink".to_string());

    Json(json!({
        "UserProfile": { "DisplayName": name },
        "RequestResult": "Success",
    }))
}

async fn handle_featured(State(library): State<Arc<Mutex<LibraryState>>>) -> Json<Value> {
    if disabled() {
        return Json(json!([]));
    }

    let books = snapshot(&library);

    Json(json!(products::newest(&books, SHELF)))
}

async fn handle_deals(State(library): State<Arc<Mutex<LibraryState>>>) -> Json<Value> {
    if disabled() {
        return Json(json!([]));
    }

    let books = snapshot(&library);

    Json(json!(products::waiting(&books, SHELF)))
}

async fn handle_recommendations(
    State(library): State<Arc<Mutex<LibraryState>>>,
) -> Json<Value> {
    if disabled() {
        return Json(json!([]));
    }

    let books = snapshot(&library);

    Json(json!(products::collecting(&books, SHELF)))
}

async fn handle_nextread(
    State(library): State<Arc<Mutex<LibraryState>>>,
    Path(book_id): Path<String>,
) -> Json<Value> {
    if disabled() {
        return Json(json!({}));
    }

    let books = snapshot(&library);
    let found = books.iter().find(|book| book.id.to_string() == book_id);

    match found.and_then(|book| products::nextread(&books, book)) {
        Some(product) => Json(product),
        None => Json(json!({})),
    }
}

async fn stub() -> (StatusCode, &'static str) {
    (StatusCode::OK, "{}")
}
