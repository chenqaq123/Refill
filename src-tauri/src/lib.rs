mod commands;
mod models;
mod services;

use services::app_state::AppState;

pub fn run() {
    install_panic_logger();
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
            commands::providers::test_provider_connection,
            commands::providers::usage_summary,
            commands::providers::read_proxy_log,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::switch::switch_profile,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Refill");
}

/// Record panics to ~/.codex-profiles/_shared-history/crash.log so failures
/// in the field leave a trace, while still printing to stderr.
fn install_panic_logger() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(home) = dirs::home_dir() {
            let dir = home.join(".codex-profiles").join("_shared-history");
            let _ = std::fs::create_dir_all(&dir);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("crash.log"))
            {
                use std::io::Write;
                let _ = writeln!(file, "{} PANIC v{} {info}", chrono::Utc::now().to_rfc3339(), env!("CARGO_PKG_VERSION"));
            }
        }
        default(info);
    }));
}
