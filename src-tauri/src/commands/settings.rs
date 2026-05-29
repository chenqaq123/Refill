use tauri::State;

use crate::models::AppSettings;
use crate::services::app_state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let mut current = state.settings.lock().map_err(|error| error.to_string())?;
    *current = settings.clone();
    Ok(settings)
}
