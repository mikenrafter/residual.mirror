//! Short-lived, local authorization for sidecar-backed metadata mutations.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use std::path::PathBuf;

use crate::config::Config;
use crate::storage::git_sidecar::SidecarConfig;

pub const MARKER_FILE: &str = ".residual-write-authorized";
pub const DEFAULT_MINUTES: i64 = 30;

pub fn marker_path(cfg: &Config) -> PathBuf {
    cfg.config_host_dir.join(MARKER_FILE)
}

/// Grant local write permission for a bounded period. This marker is deliberately
/// not ignored: a staged marker is a leak and the pre-commit hook tells the operator
/// to unstage it.
pub fn authorize(cfg: &Config, minutes: i64) -> Result<DateTime<Utc>> {
    if minutes <= 0 {
        bail!("write authorization duration must be positive minutes");
    }
    std::fs::create_dir_all(&cfg.config_host_dir)
        .with_context(|| format!("create {}", cfg.config_host_dir.display()))?;
    let expires_at = Utc::now() + Duration::minutes(minutes);
    std::fs::write(
        marker_path(cfg),
        format!("expires_at={}\n", expires_at.to_rfc3339()),
    )
    .with_context(|| format!("write {}", marker_path(cfg).display()))?;
    Ok(expires_at)
}

/// Require a current authorization only when metadata lives on a sidecar branch.
/// Inline ledgers retain their existing direct-CLI workflow.
pub fn require(cfg: &Config) -> Result<()> {
    let sidecar = SidecarConfig::from_config_file(&cfg.config_path)?;
    if !sidecar.enabled {
        return Ok(());
    }

    let path = marker_path(cfg);
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "residual metadata writes require explicit authorization; run `residual write authorize` before mutating metadata (missing {})",
            path.display()
        )
    })?;
    let raw_expiry = content
        .lines()
        .find_map(|line| line.strip_prefix("expires_at="))
        .context("invalid write-authorization marker; rerun `residual write authorize`")?;
    let expires_at = DateTime::parse_from_rfc3339(raw_expiry)
        .context("invalid write-authorization expiry; rerun `residual write authorize`")?
        .with_timezone(&Utc);
    if Utc::now() >= expires_at {
        bail!(
            "residual write authorization expired at {}; run `residual write authorize` again",
            expires_at.to_rfc3339()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn authorization_expires_and_is_required_for_sidecar() {
        let dir = tempdir().unwrap();
        let cfg = Config::for_test_residual_dir(dir.path());
        std::fs::write(
            dir.path().join("config.toml"),
            "[storage]\ngit_sidecar_enabled = true\n",
        )
        .unwrap();
        assert!(require(&cfg).is_err());
        authorize(&cfg, DEFAULT_MINUTES).unwrap();
        assert!(require(&cfg).is_ok());
        std::fs::write(
            marker_path(&cfg),
            format!(
                "expires_at={}\n",
                (Utc::now() - Duration::minutes(1)).to_rfc3339()
            ),
        )
        .unwrap();
        assert!(require(&cfg).unwrap_err().to_string().contains("expired"));
    }
}
