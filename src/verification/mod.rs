//! Verification — criticality, link + lexicon continuity, one-way tags,
//! personas adequacy (min:2 until alpha/beta exist).
//!
//! Policy (super_strict, token_warn) is read from storage-config.
//! There is no verification-config / verification/config.rs module.

use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::VerifyCheck;
use crate::config::Config;
use crate::storage::config::{self as storage_config, StorageConfig};
use crate::storage::git_sidecar::{self, SidecarConfig};

pub mod commit_msg;
pub mod git_hook;
pub mod walk_reminder;

#[allow(unused_imports)]
pub use crate::verify::{
    check_links, check_outcomes, parse_outcome, outcome_uses_terminology, LinkViolation,
    OutcomeParts, OutcomeViolation,
};

pub fn run(cfg: &Config, check: VerifyCheck) -> Result<()> {
    if let VerifyCheck::WalkReminder { staged } = &check {
        return run_walk_reminder(cfg, *staged);
    }
    if let Some(warning) = sidecar_working_tree_warning()? {
        eprintln!(
            "warning: staged residual/ paths on working branch ({}) — policy={:?}",
            warning.staged_paths.join(", "),
            warning.policy
        );
    }
    crate::verify::run(cfg, check)
}

/// Residual metadata directory for verify reads — sidecar branch tip when enabled.
pub fn metadata_dir_for_verify(cfg: &Config) -> Result<PathBuf> {
    crate::storage::metadata_dir(cfg)
}

fn sidecar_working_tree_warning() -> Result<Option<git_sidecar::WorkingTreeWarning>> {
    let cwd = std::env::current_dir()?;
    let discovery = git_sidecar::discover_config(&cwd)?;
    let sidecar = SidecarConfig::from_config_file(&discovery.config_path)?;
    git_sidecar::check_working_tree_policy(&cwd, &sidecar)
}

const DEFAULT_WALK_REMINDER_INTERVAL_DAYS: u32 = 30;

/// Non-blocking walk cadence check — always exits OK, prints reminders to stderr.
pub fn run_walk_reminder(cfg: &Config, _staged: bool) -> Result<()> {
    let meta_dir = metadata_dir_for_verify(cfg)?;
    let report = walk_reminder::verify_reminder(&meta_dir, DEFAULT_WALK_REMINDER_INTERVAL_DAYS)?;
    for msg in &report.messages {
        eprintln!("{msg}");
    }
    Ok(())
}

/// Load verification policy from storage-config (config.toml on disk, or defaults).
pub fn policy_from_storage_config(residual_dir: &Path) -> Result<StorageConfig> {
    policy_from_config_path(&residual_dir.join("config.toml"))
}

pub fn policy_from_config(cfg: &Config) -> Result<StorageConfig> {
    policy_from_config_path(&cfg.config_path)
}

pub fn policy_from_config_path(config_path: &Path) -> Result<StorageConfig> {
    let path = config_path;
    if !path.exists() {
        return Ok(StorageConfig::default());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.contains("format_version") || raw.contains("[verification]") || raw.contains("[storage]")
    {
        storage_config::parse_v3(&raw)
    } else {
        // Naive config: map validation.strict / skills.token_warn.
        let migrated = crate::storage::integrity::migration::migrate_naive_to_v3(&raw)?;
        Ok(migrated.storage)
    }
}

/// One-way tag rule: metadata MAY exist without codebase tags;
/// if tagged in code, MUST exist in metadata.
pub fn check_one_way_tags<S, T>(metadata_ids: &[S], tagged_ids: &[T]) -> Result<()>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    let meta: HashSet<String> = metadata_ids
        .iter()
        .map(|s| s.as_ref().to_string())
        .collect();
    let mut missing = Vec::new();
    for id in tagged_ids {
        let id = id.as_ref();
        if !meta.contains(id) {
            missing.push(id.to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        bail!(
            "tagged in code but missing from metadata: {}",
            missing.join(", ")
        )
    }
}

/// Until longitudinal alpha/beta exist, walks/sessions require min:2 personas
/// (stakeholder + direct user). Personas module stores; Verification checks.
pub fn check_personas_adequacy<S: AsRef<str>>(names: &[S]) -> Result<()> {
    if names.len() < 2 {
        bail!(
            "verification requires at least 2 personas (stakeholder + direct user) until alpha/beta exist; found {}",
            names.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn verification_allows_metadata_without_tags_but_not_tags_without_metadata() {
        let meta = ["S-01", "S-02"];
        let none: [&str; 0] = [];
        assert!(
            check_one_way_tags(&meta, &none).is_ok(),
            "metadata-only ids are allowed"
        );
        assert!(
            check_one_way_tags(&meta, &["S-01"]).is_ok(),
            "tagged ids that exist in metadata are allowed"
        );
        let err = check_one_way_tags(&meta, &["S-99"]).unwrap_err();
        assert!(
            err.to_string().contains("S-99"),
            "code tags without metadata must fail, got {err}"
        );
    }

    #[test]
    fn verification_requires_min_two_personas() {
        let empty: [&str; 0] = [];
        assert!(check_personas_adequacy(&empty).is_err());
        assert!(check_personas_adequacy(&["alice"]).is_err());
        assert!(check_personas_adequacy(&["alice", "bob"]).is_ok());
    }

    #[test]
    fn verification_reads_policy_from_storage_config() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();
        std::fs::write(
            residual.join("config.toml"),
            r#"
format_version = "v3"
[storage]
change_detection = true
[verification]
super_strict = false
token_warn = 321
"#,
        )
        .unwrap();
        let policy = policy_from_storage_config(&residual).unwrap();
        assert!(!policy.super_strict);
        assert_eq!(policy.token_warn, 321);
        assert!(
            !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/verification/config.rs")
                .exists(),
            "verification-config module must not exist"
        );
    }
}
