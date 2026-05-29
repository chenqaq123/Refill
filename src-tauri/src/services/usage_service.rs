use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;
use walkdir::WalkDir;

use crate::models::{RawUsageSnapshot, RawUsageWindow, UsageSnapshot};

pub fn read_usage_cache(profile_dir: &Path) -> Option<RawUsageSnapshot> {
    let data = fs::read(profile_dir.join(".codex-switcher").join("usage.json")).ok()?;
    serde_json::from_slice(&data).ok()
}

pub fn write_usage_cache(profile_dir: &Path, snapshot: &RawUsageSnapshot) -> Result<(), String> {
    let dir = profile_dir.join(".codex-switcher");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let data = serde_json::to_vec(snapshot).map_err(|error| error.to_string())?;
    fs::write(dir.join("usage.json"), data).map_err(|error| error.to_string())
}

pub fn effective(snapshot: RawUsageSnapshot) -> UsageSnapshot {
    snapshot.into_effective(Utc::now())
}

pub fn usage_snapshots(root: &Path) -> Vec<RawUsageSnapshot> {
    if !root.exists() {
        return Vec::new();
    }

    let mut snapshots = Vec::new();
    for entry in WalkDir::new(root).into_iter().flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        for line in content
            .lines()
            .filter(|line| line.contains("\"rate_limits\""))
        {
            if let Some(snapshot) = usage_snapshot_from_line(line) {
                snapshots.push(snapshot);
            }
        }
    }
    snapshots
}

pub fn newest_snapshot(
    root: &Path,
    activated_at: Option<DateTime<Utc>>,
) -> Option<RawUsageSnapshot> {
    usage_snapshots(root)
        .into_iter()
        .filter(|snapshot| {
            let Some(activated_at) = activated_at else {
                return true;
            };
            parse_date(&snapshot.timestamp)
                .map(|date| date >= activated_at)
                .unwrap_or(false)
        })
        .max_by(|left, right| left.timestamp.cmp(&right.timestamp))
}

pub fn newest_backup_snapshot(backups_dir: &Path, profile_id: &str) -> Option<RawUsageSnapshot> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(backups_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.starts_with(&format!("{profile_id}-sessions-")) {
                roots.push(path);
            }
        }
    }

    roots
        .iter()
        .flat_map(|root| usage_snapshots(root))
        .max_by(|left, right| left.timestamp.cmp(&right.timestamp))
}

fn usage_snapshot_from_line(line: &str) -> Option<RawUsageSnapshot> {
    let root: Value = serde_json::from_str(line).ok()?;
    let timestamp = root.get("timestamp")?.as_str()?.to_string();
    let rate_limits = root.get("payload")?.get("rate_limits")?;
    Some(RawUsageSnapshot {
        timestamp,
        primary: usage_window(rate_limits.get("primary")),
        secondary: usage_window(rate_limits.get("secondary")),
    })
}

fn usage_window(value: Option<&Value>) -> Option<RawUsageWindow> {
    let value = value?;
    Some(RawUsageWindow {
        used_percent: value.get("used_percent")?.as_f64()?,
        window_minutes: value.get("window_minutes")?.as_i64()?,
        resets_at: value.get("resets_at").and_then(Value::as_f64),
    })
}

pub fn parse_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}
