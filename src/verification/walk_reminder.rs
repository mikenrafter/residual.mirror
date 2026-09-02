//! Walk cadence reminders — purpose-walk and stressor-walk overdue prompts (P-23, S-46).

use anyhow::Result;
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
}

#[derive(Debug, Clone, Default)]
pub struct WalkReviewState {
    pub last_completed_purpose: Option<String>,
    pub last_completed_stressor: Option<String>,
    pub last_prompted_purpose: Option<String>,
    pub last_prompted_stressor: Option<String>,
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

pub fn load_state(residual_dir: &Path) -> Result<WalkReviewState> {
    let _ = residual_dir;
    todo!("load .walk-review.toml from effective residual dir")
}

pub fn record_completed(residual_dir: &Path, kind: WalkKind) -> Result<()> {
    let _ = (residual_dir, kind);
    todo!("stamp last_completed for purpose|stressor walk")
}

/// Non-blocking verify — always succeeds but may print reminder copy.
pub fn verify_reminder(
    residual_dir: &Path,
    interval_days: u32,
) -> Result<WalkReminderReport> {
    let _ = (residual_dir, interval_days);
    todo!("verify walk-reminder: exit 0 with overdue prompts for both walks")
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
