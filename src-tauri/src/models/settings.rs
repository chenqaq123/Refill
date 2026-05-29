use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub refresh_interval_seconds: u64,
    pub share_history: bool,
    pub codex_app_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 60,
            share_history: true,
            codex_app_name: "Codex".to_string(),
        }
    }
}
