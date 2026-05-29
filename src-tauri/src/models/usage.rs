use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUsageSnapshot {
    pub timestamp: String,
    pub primary: Option<RawUsageWindow>,
    pub secondary: Option<RawUsageWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUsageWindow {
    #[serde(alias = "used_percent")]
    pub used_percent: f64,
    #[serde(alias = "window_minutes")]
    pub window_minutes: i64,
    #[serde(alias = "resets_at")]
    pub resets_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub timestamp: String,
    pub primary: Option<UsageWindow>,
    pub secondary: Option<UsageWindow>,
    pub has_estimated_recovery: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub label: String,
    pub remaining_percent: f64,
    pub resets_at: Option<f64>,
    pub reset_text: String,
    pub is_estimated_recovered: bool,
}

impl RawUsageWindow {
    pub fn into_effective(self, now: DateTime<Utc>) -> UsageWindow {
        let is_reset = self
            .resets_at
            .and_then(|value| Utc.timestamp_opt(value as i64, 0).single())
            .map(|date| date <= now)
            .unwrap_or(false);
        let remaining = if is_reset {
            100.0
        } else {
            (100.0 - self.used_percent).clamp(0.0, 100.0)
        };

        UsageWindow {
            label: usage_label(self.window_minutes),
            remaining_percent: remaining,
            resets_at: self.resets_at,
            reset_text: reset_text(self.resets_at, now, is_reset),
            is_estimated_recovered: is_reset,
        }
    }
}

pub fn usage_label(minutes: i64) -> String {
    if minutes >= 10_080 {
        "7d".to_string()
    } else if minutes >= 1_440 {
        format!("{}d", minutes / 1_440)
    } else if minutes >= 60 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}m", minutes)
    }
}

fn reset_text(resets_at: Option<f64>, now: DateTime<Utc>, is_reset: bool) -> String {
    let Some(value) = resets_at else {
        return "重置时间未知".to_string();
    };
    let Some(reset) = Utc.timestamp_opt(value as i64, 0).single() else {
        return "重置时间未知".to_string();
    };
    if is_reset {
        return "预计已恢复".to_string();
    }

    let diff = reset.signed_duration_since(now);
    let relative = if diff.num_days() > 0 {
        format!("{}天后", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("{}小时后", diff.num_hours())
    } else if diff.num_minutes() > 0 {
        format!("{}分钟后", diff.num_minutes())
    } else {
        "即将".to_string()
    };
    let local = reset.with_timezone(&Local).format("%H:%M");
    format!("{relative} · {local}")
}

impl RawUsageSnapshot {
    pub fn into_effective(self, now: DateTime<Utc>) -> UsageSnapshot {
        let primary = self.primary.map(|window| window.into_effective(now));
        let secondary = self.secondary.map(|window| window.into_effective(now));
        let has_estimated_recovery = primary
            .as_ref()
            .map(|window| window.is_estimated_recovered)
            .unwrap_or(false)
            || secondary
                .as_ref()
                .map(|window| window.is_estimated_recovered)
                .unwrap_or(false);
        UsageSnapshot {
            timestamp: self.timestamp,
            primary,
            secondary,
            has_estimated_recovery,
        }
    }
}
