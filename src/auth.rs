use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tracing::{info, warn};
use uuid::Uuid;

static API_KEY: OnceLock<String> = OnceLock::new();

pub fn api_key() -> &'static str {
    API_KEY.get_or_init(resolve)
}

pub fn public_base() -> String {
    std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn log_endpoint() {
    let base = public_base();
    let key = api_key();

    info!("--------------------------------------------------------------");
    info!(" api key:      {key}");
    info!(" api_endpoint: {base}/{key}/kobo");
    info!("--------------------------------------------------------------");
}

fn resolve() -> String {
    if let Some(key) = from_env() {
        info!("api key loaded from API_KEY");
        return key;
    }

    let path = key_file();
    if let Some(key) = read_key(&path) {
        info!("api key loaded from {}", path.display());
        return key;
    }

    let key = Uuid::new_v4().simple().to_string();
    match persist(&path, &key) {
        Ok(()) => info!("api key generated and saved to {}", path.display()),
        Err(e) => warn!(
            "api key generated but could not be saved to {}: {e}; it will change on restart",
            path.display()
        ),
    }
    key
}

fn from_env() -> Option<String> {
    std::env::var("API_KEY")
        .ok()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

fn key_file() -> PathBuf {
    PathBuf::from(
        std::env::var("API_KEY_FILE").unwrap_or_else(|_| "/app/data/api-key".to_string()),
    )
}

fn read_key(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

fn persist(path: &Path, key: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{key}\n"))
}
