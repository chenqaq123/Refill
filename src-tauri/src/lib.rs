mod commands;
mod models;
mod services;

use services::app_state::AppState;

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            use tauri::Manager;
            let state = app.state::<AppState>();
            services::proxy::start(state.store.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::profiles::list_profiles,
            commands::profiles::refresh_profiles,
            commands::profiles::save_current_profile,
            commands::profiles::open_login_terminal,
            commands::providers::create_provider,
            commands::providers::update_provider,
            commands::providers::delete_provider,
            commands::providers::validate_provider,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::switch::switch_profile,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Refill");
}
