use serde::Serialize;

use super::{ApiProvider, UsageSnapshot};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardState {
    pub profiles: Vec<Profile>,
    pub unmanaged_current: Option<Profile>,
    pub active_label: String,
    pub profile_root: String,
    pub codex_home: String,
    pub shared_history_root: String,
    pub last_synced_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub primary_pill: String,
    pub is_active: bool,
    pub is_ready: bool,
    pub usage: Option<UsageSnapshot>,
    pub provider: Option<ApiProvider>,
    pub diagnostics: ProfileDiagnostics,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDiagnostics {
    pub profile_path: String,
    pub codex_home_path: String,
    pub sessions_shared: bool,
    pub session_index_shared: bool,
    pub desktop_state_shared: bool,
    pub workspace_shared: bool,
    pub config_exists: bool,
    pub auth_exists: bool,
    pub keychain_ready: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AccountInfo {
    pub name: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub account_id: Option<String>,
    pub subscription_until: Option<String>,
}

impl AccountInfo {
    pub fn title(&self) -> String {
        self.email
            .clone()
            .or_else(|| self.name.clone())
            .or_else(|| {
                self.account_id
                    .as_ref()
                    .map(|id| format!("Account {}", &id[..id.len().min(8)]))
            })
            .unwrap_or_else(|| "Unknown account".to_string())
    }

    pub fn subtitle(&self) -> String {
        // Account id is kept for matching but intentionally not surfaced in the
        // UI (opaque identifier, and avoids leaking it in shared screenshots).
        match &self.name {
            Some(name) if Some(name) != self.email.as_ref() && !name.is_empty() => name.clone(),
            _ => "ChatGPT 账号".to_string(),
        }
    }

    pub fn plan_display(&self) -> String {
        let Some(plan) = &self.plan else {
            return "Unknown".to_string();
        };
        if plan.eq_ignore_ascii_case("plus") {
            if let Some(until) = &self.subscription_until {
                if until.len() >= 10 {
                    return format!("Plus 至 {}", &until[5..10]);
                }
            }
        }
        let mut chars = plan.chars();
        match chars.next() {
            Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
            None => "Unknown".to_string(),
        }
    }

    pub fn matches(&self, other: &AccountInfo) -> bool {
        if let (Some(left), Some(right)) = (&self.account_id, &other.account_id) {
            if !left.is_empty() {
                return left == right;
            }
        }
        if let (Some(left), Some(right)) = (&self.email, &other.email) {
            if !left.is_empty() {
                return left.eq_ignore_ascii_case(right);
            }
        }
        false
    }
}
