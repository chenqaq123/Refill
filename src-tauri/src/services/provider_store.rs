use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::models::{ApiProviderConfig, ProviderInput, ProviderUpdateInput};

use super::shell;

#[derive(Default)]
pub struct ProviderStore;

impl ProviderStore {
    pub fn provider_config_path(profile_dir: &Path) -> PathBuf {
        profile_dir.join(".codex-switcher").join("provider.json")
    }

    pub fn read_provider(&self, profile_dir: &Path) -> Option<ApiProviderConfig> {
        let data = fs::read(Self::provider_config_path(profile_dir)).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn is_provider_profile(&self, profile_dir: &Path) -> bool {
        Self::provider_config_path(profile_dir).exists()
    }

    pub fn create_provider(
        &self,
        root: &Path,
        input: ProviderInput,
        template: Option<&Path>,
    ) -> Result<String, String> {
        let id = unique_provider_id(root, &input.name);
        let provider_id = format!("switcher-{id}");
        let profile_dir = root.join(&id);
        let config = ApiProviderConfig {
            id: id.clone(),
            name: input.name.trim().to_string(),
            base_url: normalized_base_url(&input.base_url),
            model: input.model.trim().to_string(),
            provider_id,
            wire_api: normalized_wire_api(input.wire_api.as_deref()),
            created_at: Utc::now().to_rfc3339(),
        };

        fs::create_dir_all(profile_dir.join(".codex-switcher"))
            .map_err(|error| error.to_string())?;
        self.write_key(&input.api_key, &config)?;
        self.write_provider(&config, &profile_dir)?;
        self.write_codex_config(&config, &profile_dir, template)?;
        self.create_placeholders(&profile_dir)?;
        Ok(id)
    }

    pub fn update_provider(
        &self,
        profile_dir: &Path,
        input: ProviderUpdateInput,
    ) -> Result<ApiProviderConfig, String> {
        let current = self
            .read_provider(profile_dir)
            .ok_or_else(|| "不是 API provider profile".to_string())?;
        let updated = ApiProviderConfig {
            id: current.id,
            name: input.name.trim().to_string(),
            base_url: normalized_base_url(&input.base_url),
            model: input.model.trim().to_string(),
            provider_id: current.provider_id,
            wire_api: normalized_wire_api(input.wire_api.as_deref().or(Some(&current.wire_api))),
            created_at: current.created_at,
        };

        if let Some(key) = input.api_key {
            if !key.trim().is_empty() {
                self.write_key(&key, &updated)?;
            }
        }
        self.write_provider(&updated, profile_dir)?;
        self.write_codex_config(&updated, profile_dir, Some(profile_dir))?;
        self.create_placeholders(profile_dir)?;
        Ok(updated)
    }

    pub fn delete_provider(&self, profile_dir: &Path) -> Result<(), String> {
        if let Some(provider) = self.read_provider(profile_dir) {
            let _ = self.delete_key(&provider);
        }
        fs::remove_dir_all(profile_dir).map_err(|error| error.to_string())
    }

    pub fn read_key(&self, config: &ApiProviderConfig) -> Option<String> {
        shell::run(
            "/usr/bin/security",
            &[
                "find-generic-password",
                "-w",
                "-s",
                &config.keychain_service(),
            ],
        )
        .ok()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
    }

    pub fn key_exists(&self, config: &ApiProviderConfig) -> bool {
        shell::run(
            "/usr/bin/security",
            &[
                "find-generic-password",
                "-w",
                "-s",
                &config.keychain_service(),
            ],
        )
        .is_ok()
    }

    fn write_provider(&self, config: &ApiProviderConfig, profile_dir: &Path) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
        fs::write(Self::provider_config_path(profile_dir), data).map_err(|error| error.to_string())
    }

    fn write_key(&self, api_key: &str, config: &ApiProviderConfig) -> Result<(), String> {
        let key = api_key.trim();
        if key.is_empty() {
            return Err("API key 不能为空。".to_string());
        }
        shell::run(
            "/usr/bin/security",
            &[
                "add-generic-password",
                "-U",
                "-s",
                &config.keychain_service(),
                "-a",
                &config.provider_id,
                "-w",
                key,
            ],
        )
        .map(|_| ())
    }

    fn delete_key(&self, config: &ApiProviderConfig) -> Result<(), String> {
        shell::run(
            "/usr/bin/security",
            &["delete-generic-password", "-s", &config.keychain_service()],
        )
        .map(|_| ())
    }

    fn write_codex_config(
        &self,
        config: &ApiProviderConfig,
        profile_dir: &Path,
        template: Option<&Path>,
    ) -> Result<(), String> {
        let config_path = profile_dir.join("config.toml");
        // When editing an existing provider profile, Codex Desktop has usually
        // added its own sections (mcp_servers, plugins, marketplaces, notify,
        // projects, …). Preserve everything except the keys we manage, so a
        // protocol/model change doesn't wipe Codex's setup. For a fresh profile
        // we only carry over project trust from the template.
        let preserved = if config_path.exists() {
            preserve_unmanaged_config(&fs::read_to_string(&config_path).unwrap_or_default(), &config.provider_id)
        } else {
            template
                .map(|path| extract_project_config(&path.join("config.toml")))
                .unwrap_or_default()
        };
        // Codex only speaks the Responses API. When the upstream provider only
        // offers Chat Completions, point Codex at the local translation proxy
        // instead of the real base_url; the proxy converts both ways.
        let effective_base_url = if config.wire_api == "chat" {
            super::proxy::upstream_url(&config.provider_id)
        } else {
            config.base_url.clone()
        };
        let text = format!(
            r#"model_provider = "{provider_id}"
model = "{model}"
model_reasoning_effort = "medium"

[model_providers.{provider_key}]
name = "{name}"
base_url = "{base_url}"
wire_api = "responses"
requires_openai_auth = false

[model_providers.{provider_key}.auth]
command = "/usr/bin/security"
args = ["find-generic-password", "-w", "-s", "{service}"]

{preserved}
"#,
            provider_id = toml_escape(&config.provider_id),
            provider_key = toml_bare_key(&config.provider_id),
            model = toml_escape(&config.model),
            name = toml_escape(&config.name),
            base_url = toml_escape(&effective_base_url),
            service = toml_escape(&config.keychain_service()),
            preserved = preserved.trim_end(),
        );
        fs::write(config_path, text).map_err(|error| error.to_string())
    }

    fn create_placeholders(&self, profile_dir: &Path) -> Result<(), String> {
        for folder in ["sessions", "log", "shell_snapshots", "tmp"] {
            fs::create_dir_all(profile_dir.join(folder)).map_err(|error| error.to_string())?;
        }
        let global = profile_dir.join(".codex-global-state.json");
        if !global.exists() {
            fs::write(global, "{}").map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn unique_provider_id(root: &Path, name: &str) -> String {
    let slug = slugify(if name.trim().is_empty() {
        "api-provider"
    } else {
        name
    });
    let mut candidate = slug.clone();
    let mut index = 2;
    while root.join(&candidate).exists() {
        candidate = format!("{slug}-{index}");
        index += 1;
    }
    candidate
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
        "api-provider".to_string()
    } else {
        slug
    }
}

fn normalized_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn normalized_wire_api(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some("chat") => "chat".to_string(),
        _ => "responses".to_string(),
    }
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn toml_bare_key(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
    {
        value.to_string()
    } else {
        format!("\"{}\"", toml_escape(value))
    }
}

fn extract_project_config(path: &Path) -> String {
    let Ok(content) = fs::read_to_string(path) else {
        return String::new();
    };
    let mut keep = Vec::new();
    let mut copying = false;
    for line in content.lines() {
        if line.starts_with("[projects.") {
            copying = true;
        } else if copying && line.starts_with('[') && !line.starts_with("[projects.") {
            copying = false;
        }
        if copying {
            keep.push(line.to_string());
        }
    }
    keep.join("\n")
}

/// Return the existing config.toml content minus the keys/tables Switcher
/// regenerates: the root `model_provider` / `model` / `model_reasoning_effort`
/// keys, and the `[model_providers.<id>]` (+ `.auth`) tables. Everything else
/// (notify, mcp_servers, plugins, marketplaces, projects, …) is kept verbatim.
fn preserve_unmanaged_config(content: &str, provider_id: &str) -> String {
    let provider_table = format!("[model_providers.{}]", toml_bare_key(provider_id));
    let auth_table = format!("[model_providers.{}.auth]", toml_bare_key(provider_id));
    let managed_root_keys = ["model_provider", "model", "model_reasoning_effort"];

    let mut keep: Vec<String> = Vec::new();
    let mut in_root = true;
    let mut skipping_table = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_root = false;
            let header = trimmed.trim();
            skipping_table = header == provider_table || header == auth_table;
            if skipping_table {
                continue;
            }
            keep.push(line.to_string());
            continue;
        }
        if skipping_table {
            continue;
        }
        if in_root {
            let key = trimmed
                .split('=')
                .next()
                .map(str::trim)
                .unwrap_or_default();
            if managed_root_keys.contains(&key) {
                continue;
            }
        }
        keep.push(line.to_string());
    }

    // Trim leading blank lines so the managed header sits flush.
    while keep.first().map(|line| line.trim().is_empty()).unwrap_or(false) {
        keep.remove(0);
    }
    keep.join("\n")
}

#[cfg(test)]
mod tests {
    use super::preserve_unmanaged_config;

    #[test]
    fn preserves_codex_sections_strips_managed() {
        let existing = r#"model_provider = "switcher-deepseek"
model = "gpt-5.5"
model_reasoning_effort = "medium"

notify = ["x", "turn-ended"]

[model_providers.switcher-deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
requires_openai_auth = false

[model_providers.switcher-deepseek.auth]
command = "/usr/bin/security"
args = ["a"]

[projects."/Users/cgx/Documents/Switcher"]
trust_level = "trusted"

[mcp_servers.node_repl]
command = "/x/node_repl"
"#;
        let kept = preserve_unmanaged_config(existing, "switcher-deepseek");
        // Managed root keys and the provider tables are gone.
        assert!(!kept.contains("model_provider ="));
        assert!(!kept.contains("model_reasoning_effort"));
        assert!(!kept.contains("[model_providers.switcher-deepseek]"));
        assert!(!kept.contains("base_url ="));
        assert!(!kept.contains("[model_providers.switcher-deepseek.auth]"));
        // Codex's own sections survive.
        assert!(kept.contains("notify = [\"x\", \"turn-ended\"]"));
        assert!(kept.contains("[projects.\"/Users/cgx/Documents/Switcher\"]"));
        assert!(kept.contains("[mcp_servers.node_repl]"));
        assert!(kept.contains("command = \"/x/node_repl\""));
    }
}
