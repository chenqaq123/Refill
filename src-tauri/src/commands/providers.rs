use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::models::{
    Profile, ProviderInput, ProviderTestInput, ProviderTestResult, ProviderUpdateInput,
    ProviderValidation,
};
use crate::services::{app_state::AppState, desktop_state};

#[tauri::command]
pub async fn create_provider(
    state: State<'_, AppState>,
    input: ProviderInput,
) -> Result<Profile, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(store.profile_root()).map_err(|error| error.to_string())?;
        if let Some(active) = store.active_profile_id() {
            let active_dir = store.profile_url(&active);
            let _ = desktop_state::ensure_shared_desktop_state(&store, &active_dir);
        }
        let template = store.active_profile_id().map(|id| store.profile_url(&id));
        let id = store.provider_store().create_provider(
            &store.profile_root(),
            input,
            template.as_deref(),
        )?;
        let dir = store.profile_url(&id);
        desktop_state::ensure_shared_history(&store, &dir)?;
        desktop_state::ensure_shared_desktop_state(&store, &dir)?;
        desktop_state::apply_shared_workspace_state(&store, &dir)?;
        store.profile_from_id(&id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    profile_id: String,
    input: ProviderUpdateInput,
) -> Result<Profile, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = store.profile_url(&profile_id);
        store.provider_store().update_provider(&dir, input)?;
        store.profile_from_id(&profile_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, AppState>, profile_id: String) -> Result<(), String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if store.active_profile_id().as_deref() == Some(&profile_id) {
            return Err("不能删除当前正在使用的 API profile。".to_string());
        }
        let dir = store.profile_url(&profile_id);
        store.provider_store().delete_provider(&dir)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn test_provider_connection(
    state: State<'_, AppState>,
    input: ProviderTestInput,
) -> Result<ProviderTestResult, String> {
    let store = state.store.clone();

    // Resolve the key: prefer the freshly typed one, otherwise fall back to the
    // key stored in Keychain for the provider being edited.
    let key = match input.api_key.as_deref().map(str::trim) {
        Some(key) if !key.is_empty() => key.to_string(),
        _ => {
            let from_keychain = input.profile_id.as_ref().and_then(|id| {
                let dir = store.profile_url(id);
                store
                    .provider_store()
                    .read_provider(&dir)
                    .and_then(|provider| store.provider_store().read_key(&provider))
            });
            match from_keychain {
                Some(key) => key,
                None => return Err("请填写 API Key 后再测试。".to_string()),
            }
        }
    };

    let base = input.base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("请填写 Base URL。".to_string());
    }
    let model = input.model.trim();
    let is_chat = input.wire_api == "chat";
    let (endpoint, body) = if is_chat {
        (
            format!("{base}/chat/completions"),
            json!({
                "model": model,
                "messages": [{ "role": "user", "content": "ping" }],
                "max_tokens": 1,
                "stream": false,
            }),
        )
    } else {
        (
            format!("{base}/responses"),
            json!({ "model": model, "input": "ping", "max_output_tokens": 16, "stream": false }),
        )
    };

    let client = reqwest::Client::new();
    let started = Instant::now();
    let response = client
        .post(&endpoint)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await;
    let latency_ms = started.elapsed().as_millis() as u64;

    match response {
        Ok(response) => {
            let status = response.status();
            let code = status.as_u16();
            let body_text = response.text().await.unwrap_or_default();
            let ok = status.is_success();
            let suggest_chat = !is_chat && code == 404;
            let message = if ok {
                format!("连接成功 · {endpoint} · {latency_ms}ms")
            } else if suggest_chat {
                "该服务没有 /responses 接口，应使用 Chat Completions（本地代理）。".to_string()
            } else {
                format!("{code} {} · {}", status.canonical_reason().unwrap_or(""), brief(&body_text))
            };
            Ok(ProviderTestResult {
                ok,
                status: code,
                latency_ms,
                message,
                suggest_chat,
            })
        }
        Err(error) => Ok(ProviderTestResult {
            ok: false,
            status: 0,
            latency_ms,
            message: format!("无法连接 {endpoint}：{error}"),
            suggest_chat: false,
        }),
    }
}

fn brief(text: &str) -> String {
    let trimmed = text.trim();
    let truncated: String = trimmed.chars().take(200).collect();
    if truncated.len() < trimmed.len() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[tauri::command]
pub async fn validate_provider(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ProviderValidation, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = store.profile_url(&profile_id);
        let Some(provider) = store.provider_store().read_provider(&dir) else {
            return Ok(ProviderValidation {
                ok: false,
                message: "不是 API provider".to_string(),
            });
        };
        let key_ready = store.provider_store().key_exists(&provider);
        Ok(ProviderValidation {
            ok: dir.join("config.toml").exists() && key_ready,
            message: if key_ready {
                "Keychain ready".to_string()
            } else {
                "Keychain 缺少 API key".to_string()
            },
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
