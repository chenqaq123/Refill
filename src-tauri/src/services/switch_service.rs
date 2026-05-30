use std::fs;
use std::os::unix::fs::symlink;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::models::{SwitchProgress, SwitchResult};

use super::{desktop_state, profile_store::ProfileStore, shell};

pub fn switch_profile(
    app: &AppHandle,
    store: &ProfileStore,
    profile_id: &str,
) -> Result<SwitchResult, String> {
    let target = store.profile_url(profile_id);
    if !target.exists() {
        return Err(format!("profile 不存在：{profile_id}"));
    }

    emit(app, profile_id, "quitting_codex", "正在退出 Codex", 8);
    store.cache_usage_for_active_profile();
    quit_codex()?;

    emit(app, profile_id, "syncing_current", "正在修复共享会话", 24);
    desktop_state::reconcile_all_profiles(store);
    store.reconcile_usage_caches();

    emit(app, profile_id, "syncing_current", "正在同步当前账号", 32);
    if let Some(active) = store.active_profile_id() {
        let active_dir = store.profile_url(&active);
        let _ = desktop_state::sync_workspace_state(store, &active_dir);
        let _ = desktop_state::ensure_shared_desktop_state(store, &active_dir);
    }

    emit(
        app,
        profile_id,
        "preparing_target",
        "正在准备目标 profile",
        48,
    );
    desktop_state::hydrate_desktop_profile(store, profile_id)?;

    emit(
        app,
        profile_id,
        "sharing_history",
        "正在共享会话和工作区",
        64,
    );
    desktop_state::ensure_shared_history(store, &target)?;
    desktop_state::ensure_shared_desktop_state(store, &target)?;
    desktop_state::apply_shared_workspace_state(store, &target)?;

    emit(
        app,
        profile_id,
        "linking_codex_home",
        "正在切换 ~/.codex",
        78,
    );
    if store.is_codex_home_symlink() {
        fs::remove_file(store.codex_home()).map_err(|error| error.to_string())?;
    } else if store.codex_home().exists() {
        return Err("当前账号还没保存成 profile。请先保存当前账号。".to_string());
    }
    symlink(&target, store.codex_home()).map_err(|error| error.to_string())?;
    store.write_activation_date(&target)?;

    emit(app, profile_id, "launching_codex", "正在启动 Codex", 90);
    launch_codex()?;

    emit(app, profile_id, "done", "切换完成", 100);
    Ok(SwitchResult {
        profile_id: profile_id.to_string(),
        launched: true,
    })
}

pub fn save_current_profile(store: &ProfileStore) -> Result<String, String> {
    let codex_home = store.codex_home();
    if !codex_home.exists() {
        return Err("~/.codex 不存在。".to_string());
    }
    if store.is_codex_home_symlink() {
        return Err("当前账号已经保存为 profile。".to_string());
    }
    let info = store.account_info(&codex_home);
    let profile_id = store.suggested_profile_id(&info);
    let target = store.profile_url(&profile_id);

    quit_codex()?;
    fs::create_dir_all(store.profile_root()).map_err(|error| error.to_string())?;
    fs::rename(&codex_home, &target).map_err(|error| error.to_string())?;
    desktop_state::ensure_shared_history(store, &target)?;
    desktop_state::ensure_shared_desktop_state(store, &target)?;
    desktop_state::sync_workspace_state(store, &target)?;
    desktop_state::apply_shared_workspace_state(store, &target)?;
    store.write_activation_date(&target)?;
    symlink(&target, &codex_home).map_err(|error| error.to_string())?;
    desktop_state::reconcile_all_profiles(store);
    store.reconcile_usage_caches();
    launch_codex()?;
    Ok(profile_id)
}

pub fn quit_codex() -> Result<(), String> {
    let _ = shell::run(
        "/usr/bin/osascript",
        &["-e", "tell application \"Codex\" to quit"],
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if shell::run("/usr/bin/pgrep", &["-x", "Codex"]).is_err() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(400));
    }
    Err("Codex 仍在运行。请手动退出 Codex 后再试。".to_string())
}

fn launch_codex() -> Result<(), String> {
    shell::run("/usr/bin/open", &["-a", "Codex"]).map(|_| ())
}

fn emit(app: &AppHandle, profile_id: &str, stage: &str, message: &str, percent: u8) {
    let _ = app.emit(
        "switch-progress",
        SwitchProgress {
            profile_id: profile_id.to_string(),
            stage: stage.to_string(),
            message: message.to_string(),
            percent: Some(percent),
        },
    );
}
