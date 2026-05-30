use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;
use walkdir::WalkDir;

use std::collections::BTreeMap;

use crate::models::{usage_label, RawUsageSnapshot, RawUsageWindow, UsageSnapshot, UsageWindowRecord};

fn history_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join(".codex-switcher").join("usage_history.json")
}

fn load_history(profile_dir: &Path) -> BTreeMap<String, UsageWindowRecord> {
    fs::read(history_path(profile_dir))
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn save_history(profile_dir: &Path, map: &BTreeMap<String, UsageWindowRecord>) {
    let dir = profile_dir.join(".codex-switcher");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(data) = serde_json::to_vec(map) {
        let _ = fs::write(history_path(profile_dir), data);
    }
}

fn upsert_window(
    map: &mut BTreeMap<String, UsageWindowRecord>,
    kind: &str,
    window: &RawUsageWindow,
    seen_at: &str,
) {
    let Some(resets_at) = window.resets_at else {
        return;
    };
    let key = format!("{kind}#{}", resets_at as i64);
    let entry = map.entry(key).or_insert_with(|| UsageWindowRecord {
        kind: kind.to_string(),
        label: usage_label(window.window_minutes),
        window_minutes: window.window_minutes,
        resets_at: Some(resets_at),
        used_percent: 0.0,
        last_seen: seen_at.to_string(),
        is_current: false,
    });
    entry.window_minutes = window.window_minutes;
    entry.label = usage_label(window.window_minutes);
    if window.used_percent >= entry.used_percent {
        entry.used_percent = window.used_percent;
    }
    if seen_at > entry.last_seen.as_str() {
        entry.last_seen = seen_at.to_string();
    }
}

/// Fold one or more snapshots into the persisted per-account window history,
/// keeping the peak `used_percent` for each distinct reset period.
pub fn record_usage_windows(profile_dir: &Path, snapshots: &[RawUsageSnapshot]) {
    if snapshots.is_empty() {
        return;
    }
    let mut map = load_history(profile_dir);
    for snapshot in snapshots {
        if let Some(window) = &snapshot.primary {
            upsert_window(&mut map, "primary", window, &snapshot.timestamp);
        }
        if let Some(window) = &snapshot.secondary {
            upsert_window(&mut map, "secondary", window, &snapshot.timestamp);
        }
    }
    // Keep the file bounded: retain the 80 most-recent periods.
    if map.len() > 80 {
        let mut keys: Vec<(String, i64)> = map
            .iter()
            .map(|(key, record)| (key.clone(), record.resets_at.unwrap_or(0.0) as i64))
            .collect();
        keys.sort_by(|a, b| b.1.cmp(&a.1));
        for (key, _) in keys.into_iter().skip(80) {
            map.remove(&key);
        }
    }
    save_history(profile_dir, &map);
}

/// Return the persisted window history, newest period first, with `is_current`
/// set for windows whose reset time is still in the future.
pub fn usage_history_records(profile_dir: &Path, now: DateTime<Utc>) -> Vec<UsageWindowRecord> {
    let now_secs = now.timestamp() as f64;
    let mut records: Vec<UsageWindowRecord> = load_history(profile_dir).into_values().collect();
    for record in &mut records {
        record.is_current = record.resets_at.map(|reset| reset > now_secs).unwrap_or(false);
    }
    records.sort_by(|a, b| {
        b.resets_at
            .unwrap_or(0.0)
            .partial_cmp(&a.resets_at.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    records
}

/// All snapshots whose timestamp falls within [start, end).
pub fn snapshots_between(
    root: &Path,
    start_at: Option<DateTime<Utc>>,
    end_before: Option<DateTime<Utc>>,
) -> Vec<RawUsageSnapshot> {
    usage_snapshots(root)
        .into_iter()
        .filter(|snapshot| {
            let Some(date) = parse_date(&snapshot.timestamp) else {
                return false;
            };
            if let Some(start_at) = start_at {
                if date < start_at {
                    return false;
                }
            }
            if let Some(end_before) = end_before {
                if date >= end_before {
                    return false;
                }
            }
            true
        })
        .collect()
}

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
    newest_snapshot_between(root, activated_at, None)
}

pub fn newest_snapshot_between(
    root: &Path,
    start_at: Option<DateTime<Utc>>,
    end_before: Option<DateTime<Utc>>,
) -> Option<RawUsageSnapshot> {
    usage_snapshots(root)
        .into_iter()
        .filter(|snapshot| {
            let Some(date) = parse_date(&snapshot.timestamp) else {
                return false;
            };
            if let Some(start_at) = start_at {
                if date < start_at {
                    return false;
                }
            }
            if let Some(end_before) = end_before {
                if date >= end_before {
                    return false;
                }
            }
            true
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

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(ts: &str, used: f64, resets: f64, win: i64) -> RawUsageSnapshot {
        RawUsageSnapshot {
            timestamp: ts.to_string(),
            primary: Some(RawUsageWindow {
                used_percent: used,
                window_minutes: win,
                resets_at: Some(resets),
            }),
            secondary: None,
        }
    }

    #[test]
    fn accumulates_peak_used_percent_per_period() {
        let dir = std::env::temp_dir().join(format!("usage-hist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Same reset period (1779876038) sampled twice -> keep the peak (46).
        // A different reset period (1780000000) -> separate record.
        record_usage_windows(
            &dir,
            &[
                snap("2026-05-27T01:00:00Z", 20.0, 1_779_876_038.0, 300),
                snap("2026-05-27T02:00:00Z", 46.0, 1_779_876_038.0, 300),
                snap("2026-05-27T03:00:00Z", 5.0, 1_780_000_000.0, 300),
            ],
        );
        // A later sample of the first period must not lower the peak.
        record_usage_windows(&dir, &[snap("2026-05-27T04:00:00Z", 30.0, 1_779_876_038.0, 300)]);

        let records = usage_history_records(&dir, Utc::now());
        assert_eq!(records.len(), 2);
        let first = records.iter().find(|r| r.resets_at == Some(1_779_876_038.0)).unwrap();
        assert_eq!(first.used_percent, 46.0);
        assert_eq!(first.label, "5h");
        // Newest period sorts first.
        assert_eq!(records[0].resets_at, Some(1_780_000_000.0));

        let _ = fs::remove_dir_all(&dir);
    }
}
