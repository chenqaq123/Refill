use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub provider_id: String,
    pub wire_api: String,
    pub key_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProviderConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub model: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    // "responses" = upstream natively speaks the OpenAI Responses API.
    // "chat" = upstream only speaks Chat Completions (e.g. DeepSeek); requests
    // are routed through the local translation proxy. Defaults to "responses"
    // so provider.json files written before this field existed keep working.
    #[serde(rename = "wireApi", default = "default_wire_api")]
    pub wire_api: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

pub fn default_wire_api() -> String {
    "responses".to_string()
}

impl ApiProviderConfig {
    pub fn keychain_service(&self) -> String {
        format!("local.codex.account-switcher.{}", self.provider_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    #[serde(default)]
    pub wire_api: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdateInput {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub wire_api: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderValidation {
    pub ok: bool,
    pub message: String,
}
