use tauri::State;

use crate::models::{Profile, ProviderInput, ProviderUpdateInput, ProviderValidation};
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
