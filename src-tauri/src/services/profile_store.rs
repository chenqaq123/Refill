use std::fs;
use std::path::{Path, PathBuf};

use base64::prelude::*;
use chrono::Utc;
use serde_json::Value;

use crate::models::{
    AccountInfo, ApiProvider, DashboardState, Profile, ProfileDiagnostics, RawUsageSnapshot,
};

use super::{provider_store::ProviderStore, usage_service};

pub struct ProfileStore {
    home: PathBuf,
    provider_store: ProviderStore,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            provider_store: ProviderStore::default(),
        }
    }
}

impl ProfileStore {
    pub fn codex_home(&self) -> PathBuf {
        self.home.join(".codex")
    }

    pub fn profile_root(&self) -> PathBuf {
        self.home.join(".codex-profiles")
    }

    pub fn shared_history_root(&self) -> PathBuf {
        self.profile_root().join("_shared-history")
    }

    pub fn shared_sessions(&self) -> PathBuf {
        self.shared_history_root().join("sessions")
    }

    pub fn shared_session_index(&self) -> PathBuf {
        self.shared_history_root().join("session_index.jsonl")
    }

    pub fn shared_desktop_state(&self) -> PathBuf {
        self.shared_history_root().join("desktop-state")
    }

    pub fn shared_workspace_state(&self) -> PathBuf {
        self.shared_history_root().join("workspaces.json")
    }

    pub fn provider_store(&self) -> &ProviderStore {
        &self.provider_store
    }

    /// Append one line to the proxy request log so users have a record of what
    /// the translation proxy forwarded. Best-effort; failures are ignored.
    ///
    /// The log is size-capped: once it reaches 1 MiB it is rotated to a single
    /// `.1` backup (replacing any previous one), so total disk use stays under
    /// ~2 MiB and old entries are discarded automatically.
    pub fn log_proxy(&self, line: &str) {
        use std::io::Write;
        const MAX_BYTES: u64 = 1024 * 1024;
        let dir = self.shared_history_root();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("proxy.log");
        if fs::metadata(&path)
            .map(|meta| meta.len() >= MAX_BYTES)
            .unwrap_or(false)
        {
            let _ = fs::rename(&path, dir.join("proxy.log.1"));
        }
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{} {}", Utc::now().to_rfc3339(), line);
        }
    }

    /// Aggregate recorded usage by provider and model for the cost dashboard.
    pub fn usage_summary(&self) -> crate::models::UsageSummary {
        use crate::models::{ModelUsage, ProviderUsage, UsageSummary};
        use std::collections::BTreeMap;

        // provider_id -> (model -> [requests, input, output])
        let mut by_provider: BTreeMap<String, BTreeMap<String, (u64, u64, u64)>> = BTreeMap::new();

        let root = self.shared_history_root();
        for name in ["usage.jsonl.1", "usage.jsonl"] {
            let Ok(content) = fs::read_to_string(root.join(name)) else {
                continue;
            };
            for line in content.lines() {
                let Ok(record) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let provider = record.get("provider").and_then(Value::as_str).unwrap_or("").to_string();
                let model = record.get("model").and_then(Value::as_str).unwrap_or("").to_string();
                let input = record.get("input").and_then(Value::as_u64).unwrap_or(0);
                let output = record.get("output").and_then(Value::as_u64).unwrap_or(0);
                let entry = by_provider.entry(provider).or_default().entry(model).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.1 += input;
                entry.2 += output;
            }
        }

        // Resolve provider display names.
        let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for (_, dir) in self.profile_directories() {
            if let Some(provider) = self.provider_store.read_provider(&dir) {
                names.insert(provider.provider_id, provider.name);
            }
        }

        let mut total_requests = 0u64;
        let mut total_input = 0u64;
        let mut total_output = 0u64;
        let mut providers = Vec::new();
        for (provider_id, models_map) in by_provider {
            let mut models = Vec::new();
            let (mut p_req, mut p_in, mut p_out) = (0u64, 0u64, 0u64);
            for (model, (req, input, output)) in models_map {
                p_req += req;
                p_in += input;
                p_out += output;
                models.push(ModelUsage { model, requests: req, input_tokens: input, output_tokens: output });
            }
            models.sort_by(|a, b| b.input_tokens.cmp(&a.input_tokens));
            total_requests += p_req;
            total_input += p_in;
            total_output += p_out;
            let name = names.get(&provider_id).cloned().unwrap_or_else(|| provider_id.clone());
            providers.push(ProviderUsage {
                provider_id,
                name,
                requests: p_req,
                input_tokens: p_in,
                output_tokens: p_out,
                models,
            });
        }
        providers.sort_by(|a, b| b.input_tokens.cmp(&a.input_tokens));

        UsageSummary {
            total_requests,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            providers,
        }
    }

    /// Return the most recent proxy-log lines (newest last), capped to `limit`.
    pub fn read_proxy_log(&self, limit: usize) -> Vec<String> {
        let path = self.shared_history_root().join("proxy.log");
        let Ok(content) = fs::read_to_string(path) else {
            return Vec::new();
        };
        let lines: Vec<&str> = content.lines().collect();
        let start = lines.len().saturating_sub(limit);
        lines[start..].iter().map(|line| line.to_string()).collect()
    }

    /// Append one structured usage record (one JSON object per line) for the
    /// cost/usage dashboard. Size-capped like the request log.
    pub fn record_usage(&self, provider_id: &str, model: &str, input_tokens: u64, output_tokens: u64) {
        use std::io::Write;
        const MAX_BYTES: u64 = 4 * 1024 * 1024;
        let dir = self.shared_history_root();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("usage.jsonl");
        if fs::metadata(&path)
            .map(|meta| meta.len() >= MAX_BYTES)
            .unwrap_or(false)
        {
            let _ = fs::rename(&path, dir.join("usage.jsonl.1"));
        }
        let record = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "provider": provider_id,
            "model": model,
            "input": input_tokens,
            "output": output_tokens,
        });
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{record}");
        }
    }

    /// Resolve a provider_id (e.g. "switcher-deepseek") to its real upstream
    /// base_url and configured model. Used by the translation proxy to forward
    /// requests and to force the provider's model regardless of what Codex's
    /// model picker sends.
    pub fn provider_upstream(&self, provider_id: &str) -> Option<(String, String)> {
        self.profile_directories()
            .into_iter()
            .find_map(|(_, dir)| {
                self.provider_store
                    .read_provider(&dir)
                    .filter(|provider| provider.provider_id == provider_id)
                    .map(|provider| (provider.base_url, provider.model))
            })
    }

    pub fn profile_url(&self, id: &str) -> PathBuf {
        self.profile_root().join(id)
    }

    pub fn refresh_dashboard(&self) -> DashboardState {
        self.cache_usage_for_active_profile();
        self.reconcile_usage_caches();
        let profiles = self.profiles();
        let unmanaged_current = self.current_unmanaged_profile();
        let active_label = profiles
            .iter()
            .find(|profile| profile.is_active)
            .map(|profile| profile.title.clone())
            .or_else(|| {
                unmanaged_current
                    .as_ref()
                    .map(|profile| profile.title.clone())
            })
            .unwrap_or_else(|| "未连接".to_string());

        DashboardState {
            profiles,
            unmanaged_current,
            active_label,
            profile_root: self.profile_root().display().to_string(),
            codex_home: self.codex_home().display().to_string(),
            shared_history_root: self.shared_history_root().display().to_string(),
            last_synced_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn profiles(&self) -> Vec<Profile> {
        let active = self.active_profile_id();
        let unmanaged_info = self.current_unmanaged_account_info();
        let mut profiles = Vec::new();

        for (id, path) in self.profile_directories() {
            let provider_config = self.provider_store.read_provider(&path);
            let is_provider = provider_config.is_some();
            let info = if let Some(provider) = &provider_config {
                AccountInfo {
                    name: Some(provider.name.clone()),
                    email: None,
                    plan: Some("api".to_string()),
                    account_id: Some(provider.provider_id.clone()),
                    subscription_until: None,
                }
            } else {
                self.account_info(&path)
            };
            let is_active = active.as_deref() == Some(&id)
                || (!is_provider
                    && unmanaged_info
                        .as_ref()
                        .map(|current| info.matches(current))
                        .unwrap_or(false));
            profiles.push(self.profile_from_parts(id, path, info, provider_config, is_active));
        }

        profiles.sort_by(|left, right| {
            right
                .is_active
                .cmp(&left.is_active)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });
        profiles
    }

    pub fn profile_directories(&self) -> Vec<(String, PathBuf)> {
        let mut profiles = Vec::new();
        if let Ok(entries) = fs::read_dir(self.profile_root()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() || !self.is_profile_directory(&path) {
                    continue;
                }
                profiles.push((entry.file_name().to_string_lossy().to_string(), path));
            }
        }
        profiles
    }

    pub fn current_unmanaged_profile(&self) -> Option<Profile> {
        let codex_home = self.codex_home();
        if !codex_home.exists() || self.is_codex_home_symlink() {
            return None;
        }
        let info = self.account_info(&codex_home);
        if self.profile_matching(&info).is_some() {
            return None;
        }
        Some(self.profile_from_parts("__current__".to_string(), codex_home, info, None, true))
    }

    pub fn current_unmanaged_account_info(&self) -> Option<AccountInfo> {
        let codex_home = self.codex_home();
        if !codex_home.exists() || self.is_codex_home_symlink() {
            return None;
        }
        Some(self.account_info(&codex_home))
    }

    pub fn active_profile_id(&self) -> Option<String> {
        let destination = fs::read_link(self.codex_home()).ok()?;
        let resolved = if destination.is_absolute() {
            destination
        } else {
            self.home.join(destination)
        };
        let root = self.profile_root();
        let relative = resolved.strip_prefix(root).ok()?;
        Some(relative.to_string_lossy().to_string())
    }

    pub fn is_codex_home_symlink(&self) -> bool {
        fs::symlink_metadata(self.codex_home())
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
    }

    pub fn is_profile_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            return false;
        };
        !name.starts_with('.')
            && name != "_shared-history"
            && (path.join("auth.json").exists() || self.provider_store.is_provider_profile(path))
    }

    pub fn account_info(&self, dir: &Path) -> AccountInfo {
        let Ok(data) = fs::read(dir.join("auth.json")) else {
            return AccountInfo::default();
        };
        let Ok(root) = serde_json::from_slice::<Value>(&data) else {
            return AccountInfo::default();
        };
        let tokens = root.get("tokens").unwrap_or(&Value::Null);
        let id_payload = tokens
            .get("id_token")
            .and_then(Value::as_str)
            .and_then(decode_jwt);
        let access_payload = tokens
            .get("access_token")
            .and_then(Value::as_str)
            .and_then(decode_jwt);
        let auth_claims = id_payload
            .as_ref()
            .and_then(|value| value.get("https://api.openai.com/auth"))
            .or_else(|| {
                access_payload
                    .as_ref()
                    .and_then(|value| value.get("https://api.openai.com/auth"))
            });
        let profile_claims = access_payload
            .as_ref()
            .and_then(|value| value.get("https://api.openai.com/profile"));

        AccountInfo {
            name: first_string(&[
                id_payload.as_ref().and_then(|value| value.get("name")),
                access_payload.as_ref().and_then(|value| value.get("name")),
            ]),
            email: first_string(&[
                id_payload.as_ref().and_then(|value| value.get("email")),
                profile_claims.and_then(|value| value.get("email")),
            ]),
            plan: first_string(&[
                auth_claims.and_then(|value| value.get("chatgpt_plan_type")),
                auth_claims.and_then(|value| value.get("plan_type")),
            ]),
            account_id: first_string(&[
                tokens.get("account_id"),
                auth_claims.and_then(|value| value.get("chatgpt_account_id")),
                auth_claims.and_then(|value| value.get("account_id")),
            ]),
            subscription_until: first_string(&[
                auth_claims.and_then(|value| value.get("chatgpt_subscription_active_until"))
            ]),
        }
    }

    pub fn latest_usage_snapshot(&self, id: &str, dir: &Path) -> Option<RawUsageSnapshot> {
        if Some(id.to_string()) == self.active_profile_id() {
            if let Some(live) = self.active_usage_snapshot_from_shared_history() {
                let _ = usage_service::write_usage_cache(dir, &live);
                return Some(live);
            }
        }
        if let Some(cached) = usage_service::read_usage_cache(dir) {
            return Some(cached);
        }
        if let Some(activated_at) = self.read_activation_date(dir) {
            let end_before = if Some(id.to_string()) == self.active_profile_id() {
                None
            } else {
                self.next_activation_after(activated_at)
            };
            if let Some(snapshot) = usage_service::newest_snapshot_between(
                &self.shared_sessions(),
                Some(activated_at),
                end_before,
            ) {
                let _ = usage_service::write_usage_cache(dir, &snapshot);
                return Some(snapshot);
            }
        }
        let latest =
            usage_service::newest_backup_snapshot(&self.shared_history_root().join("backups"), id);
        if let Some(snapshot) = &latest {
            let _ = usage_service::write_usage_cache(dir, snapshot);
        }
        latest
    }

    pub fn active_usage_snapshot_from_shared_history(&self) -> Option<RawUsageSnapshot> {
        let active = self.active_profile_id()?;
        let activated_at = self.read_activation_date(&self.profile_url(&active));
        usage_service::newest_snapshot(&self.shared_sessions(), activated_at)
    }

    pub fn cache_usage_for_active_profile(&self) {
        let Some(active) = self.active_profile_id() else {
            return;
        };
        let dir = self.profile_url(&active);
        if self.provider_store.read_provider(&dir).is_some() {
            return;
        }
        if let Some(snapshot) = self.active_usage_snapshot_from_shared_history() {
            let _ = usage_service::write_usage_cache(&dir, &snapshot);
            usage_service::record_usage_windows(&dir, std::slice::from_ref(&snapshot));
        }
    }

    /// Per-account rate-limit window history (official accounts only): each
    /// weekly / short window period with the peak percentage consumed.
    pub fn account_usage_history(&self, id: &str) -> Vec<crate::models::UsageWindowRecord> {
        let dir = self.profile_url(id);
        if self.provider_store.read_provider(&dir).is_some() {
            return Vec::new();
        }
        // Backfill from the snapshots attributable to this account's active
        // window (only one account is active at a time, so this is reliable).
        let start = self.read_activation_date(&dir);
        let end = if self.active_profile_id().as_deref() == Some(id) {
            None
        } else {
            start.and_then(|activated| self.next_activation_after(activated))
        };
        let snapshots = usage_service::snapshots_between(&self.shared_sessions(), start, end);
        usage_service::record_usage_windows(&dir, &snapshots);
        usage_service::usage_history_records(&dir, Utc::now())
    }

    pub fn reconcile_usage_caches(&self) {
        for (id, dir) in self.profile_directories() {
            if self.provider_store.read_provider(&dir).is_some() {
                continue;
            }
            let _ = self.latest_usage_snapshot(&id, &dir);
        }
    }

    pub fn read_activation_date(&self, profile_dir: &Path) -> Option<chrono::DateTime<Utc>> {
        let text = fs::read_to_string(profile_dir.join(".codex-switcher").join("activated_at.txt"))
            .ok()?;
        usage_service::parse_date(text.trim())
    }

    pub fn write_activation_date(&self, profile_dir: &Path) -> Result<(), String> {
        let dir = profile_dir.join(".codex-switcher");
        fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        fs::write(dir.join("activated_at.txt"), Utc::now().to_rfc3339())
            .map_err(|error| error.to_string())
    }

    fn next_activation_after(
        &self,
        activated_at: chrono::DateTime<Utc>,
    ) -> Option<chrono::DateTime<Utc>> {
        self.profile_directories()
            .into_iter()
            .filter_map(|(_, dir)| self.read_activation_date(&dir))
            .filter(|date| *date > activated_at)
            .min()
    }

    pub fn is_desktop_ready(&self, dir: &Path) -> bool {
        if let Some(provider) = self.provider_store.read_provider(dir) {
            return dir.join("config.toml").exists()
                && dir.join(".codex-global-state.json").exists()
                && self.provider_store.key_exists(&provider);
        }
        dir.join("auth.json").exists()
            && dir.join("config.toml").exists()
            && dir.join("computer-use").exists()
            && dir.join(".codex-global-state.json").exists()
    }

    pub fn profile_matching(&self, info: &AccountInfo) -> Option<PathBuf> {
        let entries = fs::read_dir(self.profile_root()).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.join("auth.json").exists() {
                continue;
            }
            if self.account_info(&path).matches(info) {
                return Some(path);
            }
        }
        None
    }

    pub fn suggested_profile_id(&self, info: &AccountInfo) -> String {
        let base = info
            .email
            .as_ref()
            .or(info.name.as_ref())
            .or(info.account_id.as_ref())
            .map(String::as_str)
            .unwrap_or("codex-account");
        let mut slug = slugify(base);
        if let Some(plan) = &info.plan {
            if !plan.is_empty() {
                slug.push('-');
                slug.push_str(&plan.to_lowercase());
            }
        }
        let mut candidate = slug.clone();
        let mut index = 2;
        while self.profile_url(&candidate).exists() {
            candidate = format!("{slug}-{index}");
            index += 1;
        }
        candidate
    }

    pub fn profile_from_id(&self, id: &str) -> Result<Profile, String> {
        let dir = self.profile_url(id);
        if !dir.exists() {
            return Err(format!("profile 不存在：{id}"));
        }
        let provider = self.provider_store.read_provider(&dir);
        let info = if let Some(provider) = &provider {
            AccountInfo {
                name: Some(provider.name.clone()),
                email: None,
                plan: Some("api".to_string()),
                account_id: Some(provider.provider_id.clone()),
                subscription_until: None,
            }
        } else {
            self.account_info(&dir)
        };
        Ok(self.profile_from_parts(
            id.to_string(),
            dir,
            info,
            provider,
            self.active_profile_id().as_deref() == Some(id),
        ))
    }

    fn profile_from_parts(
        &self,
        id: String,
        path: PathBuf,
        info: AccountInfo,
        provider_config: Option<crate::models::ApiProviderConfig>,
        is_active: bool,
    ) -> Profile {
        let provider = provider_config.as_ref().map(|config| ApiProvider {
            id: config.id.clone(),
            name: config.name.clone(),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            provider_id: config.provider_id.clone(),
            wire_api: config.wire_api.clone(),
            key_status: if self.provider_store.key_exists(config) {
                "exists".to_string()
            } else {
                "missing".to_string()
            },
            created_at: config.created_at.clone(),
        });
        let is_provider = provider.is_some();
        Profile {
            id: id.clone(),
            kind: if is_provider { "api" } else { "official" }.to_string(),
            title: provider
                .as_ref()
                .map(|provider| provider.name.clone())
                .unwrap_or_else(|| info.title()),
            subtitle: provider
                .as_ref()
                .map(|provider| {
                    let host = provider
                        .base_url
                        .split("//")
                        .last()
                        .unwrap_or(&provider.base_url)
                        .trim_end_matches('/');
                    format!("{} · {}", provider.model, host)
                })
                .unwrap_or_else(|| info.subtitle()),
            primary_pill: match provider.as_ref() {
                Some(provider) if provider.wire_api == "chat" => "Chat · 本地代理".to_string(),
                Some(_) => "Responses API".to_string(),
                None => info.plan_display(),
            },
            is_active,
            is_ready: self.is_desktop_ready(&path),
            usage: if is_provider {
                None
            } else {
                self.latest_usage_snapshot(&id, &path)
                    .map(usage_service::effective)
            },
            provider,
            diagnostics: self.diagnostics(&path, is_provider),
        }
    }

    fn diagnostics(&self, path: &Path, is_provider: bool) -> ProfileDiagnostics {
        ProfileDiagnostics {
            profile_path: path.display().to_string(),
            codex_home_path: self.codex_home().display().to_string(),
            sessions_shared: is_symlink_to(&path.join("sessions"), &self.shared_sessions()),
            session_index_shared: is_symlink_to(
                &path.join("session_index.jsonl"),
                &self.shared_session_index(),
            ),
            desktop_state_shared: self.desktop_state_shared(path),
            workspace_shared: self.shared_workspace_state().exists(),
            config_exists: path.join("config.toml").exists(),
            auth_exists: path.join("auth.json").exists(),
            keychain_ready: if is_provider {
                self.provider_store
                    .read_provider(path)
                    .map(|provider| self.provider_store.key_exists(&provider))
                    .unwrap_or(false)
            } else {
                false
            },
        }
    }

    fn desktop_state_shared(&self, path: &Path) -> bool {
        fs::read_dir(path)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.starts_with("state_")
                    && name.ends_with(".sqlite")
                    && is_symlink_to(&entry.path(), &self.shared_desktop_state().join(name))
            })
    }
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().flatten().find_map(|value| {
        value
            .as_str()
            .map(str::to_string)
            .filter(|value| !value.is_empty())
    })
}

fn decode_jwt(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let data = BASE64_URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&data).ok()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if matches!(character, '@' | '.' | '_' | '-') {
            slug.push('-');
        } else {
            slug.push('-');
        }
    }
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "codex-account".to_string()
    } else {
        slug
    }
}

fn is_symlink_to(path: &Path, expected: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    fs::read_link(path)
        .map(|target| target == expected)
        .unwrap_or(false)
}
