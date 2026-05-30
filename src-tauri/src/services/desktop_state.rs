use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{profile_store::ProfileStore, shell};

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceState {
    saved_workspace_roots: Vec<String>,
    project_order: Vec<String>,
    active_workspace_roots: Vec<String>,
}

impl WorkspaceState {
    fn is_empty(&self) -> bool {
        self.saved_workspace_roots.is_empty()
            && self.project_order.is_empty()
            && self.active_workspace_roots.is_empty()
    }
}

pub fn ensure_shared_history(store: &ProfileStore, profile_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(store.shared_sessions()).map_err(|error| error.to_string())?;

    let local_sessions = profile_dir.join("sessions");
    if !is_symlink_to(&local_sessions, &store.shared_sessions()) {
        copy_dir_contents_if_missing(&local_sessions, &store.shared_sessions())?;
        if is_symlink(&local_sessions) {
            fs::remove_file(&local_sessions).map_err(|error| error.to_string())?;
        } else {
            backup_and_remove(
                store,
                &local_sessions,
                &format!("{}-sessions", profile_name(profile_dir)),
            )?;
        }
        symlink(store.shared_sessions(), &local_sessions).map_err(|error| error.to_string())?;
    }

    let local_index = profile_dir.join("session_index.jsonl");
    if !is_symlink_to(&local_index, &store.shared_session_index()) {
        append_session_index_if_needed(&local_index, &store.shared_session_index())?;
        if is_symlink(&local_index) {
            fs::remove_file(&local_index).map_err(|error| error.to_string())?;
        } else {
            backup_and_remove(
                store,
                &local_index,
                &format!("{}-session-index", profile_name(profile_dir)),
            )?;
        }
        symlink(store.shared_session_index(), &local_index).map_err(|error| error.to_string())?;
    } else if !store.shared_session_index().exists() {
        fs::write(store.shared_session_index(), "").map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn reconcile_all_profiles(store: &ProfileStore) {
    for (_, profile_dir) in store.profile_directories() {
        let _ = ensure_shared_history(store, &profile_dir);
        let _ = ensure_shared_desktop_state(store, &profile_dir);
        let _ = sync_workspace_state(store, &profile_dir);
    }
    for (_, profile_dir) in store.profile_directories() {
        let _ = apply_shared_workspace_state(store, &profile_dir);
    }
}

pub fn ensure_shared_desktop_state(store: &ProfileStore, profile_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(store.shared_desktop_state()).map_err(|error| error.to_string())?;
    let mut names = sqlite_state_base_names(profile_dir);
    names.extend(sqlite_state_base_names(&store.shared_desktop_state()));

    for base_name in names {
        let local_base = profile_dir.join(&base_name);
        let shared_base = store.shared_desktop_state().join(&base_name);

        checkpoint_sqlite(&local_base);
        checkpoint_sqlite(&shared_base);

        if !shared_base.exists() {
            copy_sqlite_set_if_missing(&base_name, profile_dir, &store.shared_desktop_state())?;
        } else if base_name.starts_with("state_") && !merge_threads(&local_base, &shared_base) {
            continue;
        }

        if !shared_base.exists() {
            continue;
        }
        if !is_symlink(&local_base) {
            backup_and_remove_sqlite_set(store, &base_name, profile_dir)?;
        }
        link_sqlite_set(&base_name, profile_dir, &store.shared_desktop_state())?;
    }
    Ok(())
}

pub fn sync_workspace_state(store: &ProfileStore, profile_dir: &Path) -> Result<(), String> {
    let local = workspace_state(profile_dir);
    if local.is_empty() {
        return Ok(());
    }
    let merged = merge_workspace(local, read_shared_workspace_state(store));
    write_shared_workspace_state(store, &merged)
}

pub fn apply_shared_workspace_state(
    store: &ProfileStore,
    profile_dir: &Path,
) -> Result<(), String> {
    let shared = read_shared_workspace_state(store);
    if shared.is_empty() {
        return Ok(());
    }
    let global_path = profile_dir.join(".codex-global-state.json");
    let Ok(data) = fs::read(&global_path) else {
        return Ok(());
    };
    let mut object: serde_json::Value =
        serde_json::from_slice(&data).unwrap_or_else(|_| serde_json::json!({}));
    let local = workspace_state(profile_dir);
    let merged = merge_workspace(shared, local);
    object["electron-saved-workspace-roots"] = serde_json::json!(merged.saved_workspace_roots);
    object["project-order"] = serde_json::json!(merged.project_order);
    if !merged.active_workspace_roots.is_empty() {
        object["active-workspace-roots"] = serde_json::json!(merged.active_workspace_roots);
    }
    let data = serde_json::to_vec_pretty(&object).map_err(|error| error.to_string())?;
    fs::write(global_path, data).map_err(|error| error.to_string())
}

pub fn align_thread_provider(store: &ProfileStore, profile_dir: &Path) -> Result<(), String> {
    let provider_id = store
        .provider_store()
        .read_provider(profile_dir)
        .map(|provider| provider.provider_id)
        .unwrap_or_else(|| "openai".to_string());
    let escaped_provider = provider_id.replace('\'', "''");

    for base_name in sqlite_state_base_names(&store.shared_desktop_state()) {
        if !base_name.starts_with("state_") {
            continue;
        }
        let shared_base = store.shared_desktop_state().join(&base_name);
        if !shared_base.exists() {
            continue;
        }
        let sql = format!(
            "UPDATE threads SET model_provider = '{}' WHERE model_provider != '{}'; PRAGMA wal_checkpoint(TRUNCATE);",
            escaped_provider, escaped_provider
        );
        shell::run(
            "/usr/bin/sqlite3",
            &[&shared_base.display().to_string(), &sql],
        )
        .map(|_| ())?;
    }
    Ok(())
}

pub fn hydrate_desktop_profile(store: &ProfileStore, profile_id: &str) -> Result<(), String> {
    let target = store.profile_url(profile_id);
    if !target.exists() {
        return Err(format!("profile 不存在：{profile_id}"));
    }
    if store.is_codex_home_symlink() && store.active_profile_id().as_deref() == Some(profile_id) {
        ensure_shared_history(store, &target)?;
        ensure_shared_desktop_state(store, &target)?;
        apply_shared_workspace_state(store, &target)?;
        return Ok(());
    }
    let codex_home = store.codex_home();
    if !codex_home.exists() {
        return Err("找不到可用于初始化 Desktop profile 的 ~/.codex。".to_string());
    }

    for item in [
        ".codex-global-state.json",
        ".codex-global-state.json.bak",
        ".personality_migration",
        ".tmp",
        "ambient-suggestions",
        "cache",
        "computer-use",
        "installation_id",
        "memories",
        "models_cache.json",
        "plugins",
        "rules",
        "skills",
        "sqlite",
        "tmp",
        "vendor_imports",
        "version.json",
    ] {
        copy_if_missing(item, &codex_home, &target)?;
    }
    for folder in ["sessions", "log", "shell_snapshots"] {
        fs::create_dir_all(target.join(folder)).map_err(|error| error.to_string())?;
    }
    ensure_shared_history(store, &target)?;
    ensure_shared_desktop_state(store, &target)?;
    apply_shared_workspace_state(store, &target)
}

pub fn backup_and_remove(store: &ProfileStore, path: &Path, label: &str) -> Result<(), String> {
    if !path.exists() && !is_symlink(path) {
        return Ok(());
    }
    let backups = store.shared_history_root().join("backups");
    fs::create_dir_all(&backups).map_err(|error| error.to_string())?;
    let backup = backups.join(format!("{}-{}", label, chrono::Utc::now().timestamp()));
    fs::rename(path, backup).map_err(|error| error.to_string())
}

fn copy_if_missing(name: &str, source: &Path, target: &Path) -> Result<(), String> {
    let source_url = source.join(name);
    let target_url = target.join(name);
    if !source_url.exists() || target_url.exists() {
        return Ok(());
    }
    if source_url.is_dir() {
        copy_dir_recursive(&source_url, &target_url)
    } else {
        fs::copy(source_url, target_url)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target_path = target.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &target_path)?;
        } else if !target_path.exists() {
            fs::copy(entry.path(), target_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn copy_dir_contents_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in walkdir::WalkDir::new(source).into_iter().flatten() {
        let path = entry.path();
        if path == source {
            continue;
        }
        let Ok(relative) = path.strip_prefix(source) else {
            continue;
        };
        let target_path = target.join(relative);
        if target_path.exists() {
            continue;
        }
        if path.is_dir() {
            fs::create_dir_all(&target_path).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(path, target_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn append_session_index_if_needed(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        if !target.exists() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(target, "").map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    let existing = fs::read_to_string(target).unwrap_or_default();
    let incoming = fs::read_to_string(source).unwrap_or_default();
    let mut seen: HashSet<String> = existing.lines().map(str::to_string).collect();
    let mut merged = existing;
    for line in incoming.lines() {
        if seen.insert(line.to_string()) {
            if !merged.is_empty() && !merged.ends_with('\n') {
                merged.push('\n');
            }
            merged.push_str(line);
            merged.push('\n');
        }
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(target, merged).map_err(|error| error.to_string())
}

fn sqlite_state_base_names(dir: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let mut name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("state_") && !name.starts_with("goals_") {
                continue;
            }
            if name.ends_with("-wal") || name.ends_with("-shm") {
                name.truncate(name.len() - 4);
            }
            if name.ends_with(".sqlite") {
                names.insert(name);
            }
        }
    }
    names
}

fn checkpoint_sqlite(path: &Path) {
    if path.exists() && !is_symlink(path) {
        let _ = shell::run(
            "/usr/bin/sqlite3",
            &[
                &path.display().to_string(),
                "PRAGMA wal_checkpoint(TRUNCATE);",
            ],
        );
    }
}

fn merge_threads(source: &Path, destination: &Path) -> bool {
    if !source.exists() || !destination.exists() || is_symlink(source) {
        return true;
    }
    let sql = format!(
        "ATTACH DATABASE '{}' AS incoming; INSERT OR REPLACE INTO main.threads SELECT * FROM incoming.threads; DETACH DATABASE incoming; PRAGMA wal_checkpoint(TRUNCATE);",
        source.display().to_string().replace('\'', "''")
    );
    shell::run(
        "/usr/bin/sqlite3",
        &[&destination.display().to_string(), &sql],
    )
    .is_ok()
}

fn copy_sqlite_set_if_missing(base_name: &str, source: &Path, target: &Path) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm"] {
        let source_path = source.join(format!("{base_name}{suffix}"));
        let target_path = target.join(format!("{base_name}{suffix}"));
        if source_path.exists() && !target_path.exists() {
            fs::copy(source_path, target_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn backup_and_remove_sqlite_set(
    store: &ProfileStore,
    base_name: &str,
    dir: &Path,
) -> Result<(), String> {
    for suffix in ["-shm", "-wal", ""] {
        backup_and_remove(
            store,
            &dir.join(format!("{base_name}{suffix}")),
            &format!("{}-{base_name}{suffix}", profile_name(dir)),
        )?;
    }
    Ok(())
}

fn link_sqlite_set(base_name: &str, profile_dir: &Path, shared_dir: &Path) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm"] {
        let local = profile_dir.join(format!("{base_name}{suffix}"));
        let shared = shared_dir.join(format!("{base_name}{suffix}"));
        if is_symlink_to(&local, &shared) {
            continue;
        }
        if is_symlink(&local) {
            fs::remove_file(&local).map_err(|error| error.to_string())?;
        }
        if local.exists() {
            fs::remove_file(&local).map_err(|error| error.to_string())?;
        }
        symlink(shared, local).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn workspace_state(profile_dir: &Path) -> WorkspaceState {
    let Ok(data) = fs::read(profile_dir.join(".codex-global-state.json")) else {
        return WorkspaceState::default();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&data) else {
        return WorkspaceState::default();
    };
    WorkspaceState {
        saved_workspace_roots: string_array(value.get("electron-saved-workspace-roots")),
        project_order: string_array(value.get("project-order")),
        active_workspace_roots: string_array(value.get("active-workspace-roots")),
    }
}

fn read_shared_workspace_state(store: &ProfileStore) -> WorkspaceState {
    fs::read(store.shared_workspace_state())
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn write_shared_workspace_state(
    store: &ProfileStore,
    state: &WorkspaceState,
) -> Result<(), String> {
    fs::create_dir_all(store.shared_history_root()).map_err(|error| error.to_string())?;
    let data = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    fs::write(store.shared_workspace_state(), data).map_err(|error| error.to_string())
}

fn merge_workspace(preferred: WorkspaceState, fallback: WorkspaceState) -> WorkspaceState {
    let saved = unique(
        preferred
            .saved_workspace_roots
            .iter()
            .chain(fallback.saved_workspace_roots.iter())
            .chain(preferred.project_order.iter())
            .chain(fallback.project_order.iter())
            .cloned()
            .collect(),
    );
    let order = unique(
        preferred
            .project_order
            .iter()
            .chain(fallback.project_order.iter())
            .chain(saved.iter())
            .cloned()
            .collect(),
    );
    let active = if preferred.active_workspace_roots.is_empty() {
        fallback.active_workspace_roots
    } else {
        preferred.active_workspace_roots
    };
    WorkspaceState {
        saved_workspace_roots: saved,
        project_order: order,
        active_workspace_roots: active,
    }
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| !value.is_empty() && seen.insert(value.clone()))
        .collect()
}

fn string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn is_symlink_to(path: &Path, expected: &Path) -> bool {
    if !is_symlink(path) {
        return false;
    }
    fs::read_link(path)
        .map(|target| target == expected)
        .unwrap_or(false)
}

fn profile_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("profile")
        .to_string()
}
