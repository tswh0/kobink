#![allow(non_snake_case)]
use base64::{engine::general_purpose::STANDARD, Engine as _};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncToken {
    pub version: u32,
    pub library: u64,
}

impl SyncToken {
    pub fn new(library: u64) -> Self {
        Self {
            version: 1,
            library,
        }
    }

    pub fn encode(&self) -> String {
        STANDARD.encode(serde_json::to_vec(self).unwrap_or_default())
    }

    pub fn decode(value: &str) -> Option<Self> {
        serde_json::from_slice(&STANDARD.decode(value).ok()?).ok()
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
