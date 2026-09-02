//! Walk cadence reminders — purpose-walk and stressor-walk overdue prompts (P-23, S-46).

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkKind {
    Purpose,
    Stressor,
}

impl WalkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WalkKind::Purpose => "purpose",
            WalkKind::Stressor => "stressor",
        }
    }

    fn field_key(self) -> &'static str {
        match self {
            WalkKind::Purpose => "purpose-walk",
            WalkKind::Stressor => "stressor-walk",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WalkReviewState {
    pub last_completed_purpose: Option<String>,
    pub last_completed_stressor: Option<String>,
    pub last_prompted_purpose: Option<String>,
    pub last_prompted_stressor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WalkReviewFile {
    #[serde(default)]
    last_completed: WalkDates,
    #[serde(default)]
    last_prompted: WalkDates,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WalkDates {
    #[serde(default, rename = "purpose-walk")]
    purpose_walk: Option<String>,
    #[serde(default, rename = "stressor-walk")]
    stressor_walk: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WalkReminderReport {
    pub overdue: Vec<String>,
    pub messages: Vec<String>,
}

/// Path to .walk-review.toml on effective (sidecar) residual dir.
pub fn walk_review_path(residual_dir: &Path) -> PathBuf {
    residual_dir.join(".walk-review.toml")
}

fn parse_file(raw: &str) -> Result<WalkReviewFile> {
    toml::from_str(raw).context("parse .walk-review.toml")
}

fn state_from_file(file: &WalkReviewFile) -> WalkReviewState {
    WalkReviewState {
        last_completed_purpose: file.last_completed.purpose_walk.clone(),
        last_completed_stressor: file.last_completed.stressor_walk.clone(),
        last_prompted_purpose: file.last_prompted.purpose_walk.clone(),
        last_prompted_stressor: file.last_prompted.stressor_walk.clone(),
    }
}

pub fn load_state(residual_dir: &Path) -> Result<WalkReviewState> {
    let path = walk_review_path(residual_dir);
    if !path.exists() {
        return Ok(WalkReviewState::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(state_from_file(&parse_file(&raw)?))
}

fn write_state(residual_dir: &Path, file: &WalkReviewFile) -> Result<()> {
    let path = walk_review_path(residual_dir);
    let body = toml::to_string_pretty(file).context("serialize .walk-review.toml")?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))
}

pub fn record_completed(residual_dir: &Path, kind: WalkKind) -> Result<()> {
    let mut file = if walk_review_path(residual_dir).exists() {
        parse_file(&std::fs::read_to_string(walk_review_path(residual_dir))?)? 
    } else {
        WalkReviewFile::default()
    };
    let today = Local::now().format("%Y-%m-%d").to_string();
    match kind {
        WalkKind::Purpose => file.last_completed.purpose_walk = Some(today),
        WalkKind::Stressor => file.last_completed.stressor_walk = Some(today),
    }
    write_state(residual_dir, &file)
}

fn days_since(iso_date: &str) -> Option<i64> {
    let parsed = NaiveDate::parse_from_str(iso_date, "%Y-%m-%d").ok()?;
    let today = Local::now().date_naive();
    Some((today - parsed).num_days())
}

fn reminder_message(kind: WalkKind, days: i64, interval_days: u32) -> String {
    format!(
        "WALK REMINDER: last {}-walk {days}d ago (interval {interval_days}d). \
         → residual skill data {}-walk \
         → residual walk record --kind {} --completed|--deferred",
        kind.as_str(),
        kind.as_str(),
        kind.as_str()
    )
}

/// Non-blocking verify — always succeeds but may print reminder copy.
pub fn verify_reminder(
    residual_dir: &Path,
    interval_days: u32,
) -> Result<WalkReminderReport> {
    let state = load_state(residual_dir)?;
    let mut report = WalkReminderReport {
        overdue: Vec::new(),
        messages: Vec::new(),
    };

    for kind in [WalkKind::Purpose, WalkKind::Stressor] {
        let last = match kind {
            WalkKind::Purpose => state.last_completed_purpose.as_deref(),
            WalkKind::Stressor => state.last_completed_stressor.as_deref(),
        };
        let overdue = match last {
            None => true,
            Some(date) => days_since(date).is_none_or(|d| d >= i64::from(interval_days)),
        };
        if overdue {
            let days = last
                .and_then(days_since)
                .unwrap_or(i64::from(interval_days) + 1);
            report.overdue.push(kind.field_key().to_string());
            report.messages.push(reminder_message(kind, days, interval_days));
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local};
    use tempfile::tempdir;

    #[test]
    fn walk_review_toml_lives_on_effective_residual_dir() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();

        let path = walk_review_path(&residual);
        assert_eq!(
            path,
            residual.join(".walk-review.toml"),
            ".walk-review.toml must live on effective/sidecar residual dir"
        );
    }

    #[test]
    fn record_completed_updates_state_for_purpose_and_stressor() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();

        record_completed(&residual, WalkKind::Purpose).unwrap();
        record_completed(&residual, WalkKind::Stressor).unwrap();

        let state = load_state(&residual).unwrap();
        assert!(
            state.last_completed_purpose.is_some(),
            "purpose-walk completion must be recorded"
        );
        assert!(
            state.last_completed_stressor.is_some(),
            "stressor-walk completion must be recorded"
        );

        let toml = std::fs::read_to_string(walk_review_path(&residual)).unwrap();
        assert!(toml.contains("purpose-walk"));
        assert!(toml.contains("stressor-walk"));
    }

    #[test]
    fn verify_reminder_exits_ok_but_prints_when_overdue() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();

        let stale = (Local::now() - Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();
        std::fs::write(
            walk_review_path(&residual),
            format!(
                "[last_completed]\npurpose-walk = \"{stale}\"\nstressor-walk = \"{stale}\"\n"
            ),
        )
        .unwrap();

        let report = verify_reminder(&residual, 30).unwrap();
        assert!(
            !report.messages.is_empty() || !report.overdue.is_empty(),
            "overdue walks must produce reminder messages"
        );
        assert!(
            report.messages.iter().any(|m| m.to_lowercase().contains("purpose"))
                && report.messages.iter().any(|m| m.to_lowercase().contains("stressor")),
            "separate prompts required for purpose-walk AND stressor-walk, got: {:?}",
            report.messages
        );
    }
}
