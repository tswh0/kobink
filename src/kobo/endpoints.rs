use std::sync::{Arc, Mutex, OnceLock};

use crate::{
    auth::{api_key, public_base},
    library::models::LibraryState,
};

use super::{
    library::create_library_router,
    models::{AuthDeviceRequest, AuthDeviceResponse},
};

use axum::{
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

const INITIALIZATION_DATA: &str = include_str!("./data/initialization.json");

pub fn create_kobo_router() -> Router<Arc<Mutex<LibraryState>>> {
    Router::new()
        .route("/auth/device", post(handle_auth_device))
        .route("/user/profile", get(stub))
        .route("/user/loyalty/benefits", get(stub))
        .route("/user/recommendations", get(stub))
        .route("/deals", get(stub))
        .route("/products/{*_}", get(stub))
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

async fn stub() -> (StatusCode, &'static str) {
    (StatusCode::OK, "{}")
}
