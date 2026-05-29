use tauri::{AppHandle, State};

use crate::models::SwitchResult;
use crate::services::{app_state::AppState, switch_service};

#[tauri::command]
pub async fn switch_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<SwitchResult, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        switch_service::switch_profile(&app, &store, &profile_id)
    })
    .await
    .map_err(|error| error.to_string())?
}
