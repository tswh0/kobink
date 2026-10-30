#![allow(non_snake_case)]
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthDeviceRequest {
    pub AffiliateName: String,
    pub AppVersion: String,
    pub ClientKey: String,
    pub DeviceId: String,
    pub PlatformId: String,
    pub SerialNumber: String,
    pub UserKey: String,
}

// from calibre-web,
#[derive(Debug, Serialize, Deserialize)]
pub struct _SyncTokenData {
    pub raw_kobo_store_token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub books_last_modified: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub books_last_created: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub archive_last_modified: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub reading_state_last_modified: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub tags_last_modified: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncTokenModel {
    pub version: String,
    pub data: _SyncTokenData,
}

impl SyncTokenModel {
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            version: "1".to_string(),
            data: _SyncTokenData {
                raw_kobo_store_token: String::new(),
                books_last_modified: now,
                books_last_created: now,
                archive_last_modified: now,
                reading_state_last_modified: now,
                tags_last_modified: now,
            },
        }
    }

    pub fn to_header_value(&self) -> String {
        STANDARD.encode(serde_json::to_string(self).unwrap_or_default())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthDeviceResponse {
    pub AccessToken: String,
    pub RefreshToken: String,
    pub TokenType: String,
    pub TrackingId: String,
    pub UserKey: String,
}

impl AuthDeviceResponse {
    pub fn from_user_key(user_key: impl ToString) -> Self {
        Self {
            AccessToken: String::new(),
            RefreshToken: "SAMPLE_REFRESH_TOKEN".to_string(),
            TokenType: "Bearer".to_string(),
            TrackingId: "SAMPLE_TRACKING_ID".to_string(),
            UserKey: user_key.to_string(),
        }
    }
}
