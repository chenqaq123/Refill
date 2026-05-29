use tauri::State;

use crate::models::{DashboardState, Profile};
use crate::services::{app_state::AppState, shell, switch_service};

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || Ok(store.profiles()))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn refresh_profiles(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || Ok(store.refresh_dashboard()))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn save_current_profile(state: State<'_, AppState>) -> Result<Profile, String> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let id = switch_service::save_current_profile(&store)?;
        store.profile_from_id(&id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn open_login_terminal() -> Result<(), String> {
    let script = "/Users/cgx/Documents/Switcher/bin/codex-as";
    let generated = format!("login-{}", chrono::Utc::now().timestamp());
    let command = format!(
        "'{}' --login '{}'",
        script.replace('\'', "'\\''"),
        generated
    );
    let apple_script = format!(
        "tell application \"Terminal\"\n  activate\n  do script \"{}\"\nend tell",
        command.replace('"', "\\\"")
    );
    shell::run("/usr/bin/osascript", &["-e", &apple_script]).map(|_| ())
}
