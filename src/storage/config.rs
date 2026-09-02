//! Storage.Config — THE config TOML (format_version v4; v3 on disk migrated): app + verify policy keys.
//!
//! Owns format_version, change_detection, AND verification policy
//! (super_strict, token_warn). There is no verification-config module.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

fn default_format_version() -> String {
    "v4".to_string()
}
fn default_change_detection() -> bool {
    true
}
fn default_super_strict() -> bool {
    true
}
fn default_token_warn() -> usize {
    1000
}
fn default_commit_msg_enforce() -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_format_version")]
    pub format_version: String,
    #[serde(default = "default_change_detection")]
    pub change_detection: bool,
    #[serde(default = "default_super_strict")]
    pub super_strict: bool,
    #[serde(default = "default_token_warn")]
    pub token_warn: usize,
    /// When false, commit-msg hook prints violations but exits 0 (warn-only).
    #[serde(default = "default_commit_msg_enforce")]
    pub commit_msg_enforce: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            format_version: default_format_version(),
            change_detection: default_change_detection(),
            super_strict: default_super_strict(),
            token_warn: default_token_warn(),
            commit_msg_enforce: default_commit_msg_enforce(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct V3Document {
    #[serde(default = "default_format_version")]
    format_version: String,
    #[serde(default)]
    storage: StorageSection,
    #[serde(default)]
    verification: VerificationSection,
}

fn default_git_sidecar_branch() -> String {
    "residual/metadata".to_string()
}
fn default_git_sidecar_remote() -> String {
    "origin".to_string()
}
fn default_config_host() -> String {
    "repo".to_string()
}
fn default_working_tree_policy() -> String {
    "warn".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StorageSection {
    #[serde(default = "default_change_detection")]
    change_detection: bool,
    #[serde(default)]
    git_sidecar_enabled: bool,
    #[serde(default = "default_git_sidecar_branch")]
    git_sidecar_branch: String,
    #[serde(default = "default_git_sidecar_remote")]
    git_sidecar_remote: String,
    #[serde(default = "default_config_host")]
    config_host: String,
    #[serde(default)]
    git_sidecar: GitSidecarNested,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GitSidecarNested {
    #[serde(default = "default_working_tree_policy")]
    working_tree_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationSection {
    #[serde(default = "default_super_strict")]
    super_strict: bool,
    #[serde(default = "default_token_warn")]
    token_warn: usize,
    #[serde(default = "default_commit_msg_enforce")]
    commit_msg_enforce: bool,
}

impl Default for VerificationSection {
    fn default() -> Self {
        Self {
            super_strict: default_super_strict(),
            token_warn: default_token_warn(),
            commit_msg_enforce: default_commit_msg_enforce(),
        }
    }
}

/// Parse a v3 TOML document. App keys and verify-policy keys both land here.
pub fn parse_v3(toml_str: &str) -> Result<StorageConfig> {
    let doc: V3Document = toml::from_str(toml_str).with_context(|| "parse storage v3 TOML")?;
    Ok(StorageConfig {
        format_version: doc.format_version,
        change_detection: doc.storage.change_detection,
        super_strict: doc.verification.super_strict,
        token_warn: doc.verification.token_warn,
        commit_msg_enforce: doc.verification.commit_msg_enforce,
    })
}

/// Render the full v3 document (storage + verify policy sections).
pub fn render_v3(cfg: &StorageConfig) -> String {
    format!(
        "# residual v4 configuration\nformat_version = \"{}\"\n\n[storage]\nchange_detection = {}\n\n[verification]\nsuper_strict = {}\ntoken_warn = {}\ncommit_msg_enforce = {}\n",
        cfg.format_version, cfg.change_detection, cfg.super_strict, cfg.token_warn, cfg.commit_msg_enforce
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarStorageConfig {
    pub git_sidecar_enabled: bool,
    pub git_sidecar_branch: String,
    pub git_sidecar_remote: String,
    pub config_host: String,
    pub working_tree_policy: String,
}

/// Parse [storage] sidecar keys from config TOML.
pub fn parse_sidecar_section(toml_str: &str) -> Result<SidecarStorageConfig> {
    let doc: V3Document = toml::from_str(toml_str).with_context(|| "parse sidecar TOML")?;
    Ok(SidecarStorageConfig {
        git_sidecar_enabled: doc.storage.git_sidecar_enabled,
        git_sidecar_branch: doc.storage.git_sidecar_branch,
        git_sidecar_remote: doc.storage.git_sidecar_remote,
        config_host: doc.storage.config_host,
        working_tree_policy: doc.storage.git_sidecar.working_tree_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_config_parses_v3_toml() {
        let raw = r#"
format_version = "v3"

[storage]
change_detection = false

[verification]
super_strict = false
token_warn = 42
"#;
        let cfg = parse_v3(raw).unwrap();
        assert_eq!(cfg.format_version, "v3");
        assert!(!cfg.change_detection);
        assert!(!cfg.super_strict);
        assert_eq!(cfg.token_warn, 42);
    }

    #[test]
    fn verification_reads_policy_from_storage_config() {
        let raw = r#"
format_version = "v3"
[storage]
change_detection = true
[verification]
super_strict = true
token_warn = 777
"#;
        let cfg = parse_v3(raw).unwrap();
        // Verification consumes these fields from storage-config — no separate module.
        assert!(cfg.super_strict);
        assert_eq!(cfg.token_warn, 777);
        let rendered = render_v3(&cfg);
        assert!(rendered.contains("super_strict"));
        assert!(rendered.contains("token_warn"));
        assert!(rendered.contains("[storage]"));
    }

    #[test]
    fn parse_sidecar_section_reads_git_sidecar_enabled() {
        let raw = r#"
format_version = "v4"
[storage]
change_detection = true
git_sidecar_enabled = true
git_sidecar_branch = "residual/metadata"
git_sidecar_remote = "origin"
config_host = "parent"

[storage.git_sidecar]
working_tree_policy = "warn"

[verification]
super_strict = true
token_warn = 1000
"#;
        let sidecar = parse_sidecar_section(raw).unwrap();
        assert!(sidecar.git_sidecar_enabled);
        assert_eq!(sidecar.git_sidecar_branch, "residual/metadata");
        assert_eq!(sidecar.config_host, "parent");
        assert_eq!(sidecar.working_tree_policy, "warn");
    }

    #[test]
    fn parse_sidecar_section_defaults_disabled() {
        let raw = r#"
format_version = "v4"
[storage]
change_detection = true
[verification]
super_strict = true
token_warn = 1000
"#;
        let sidecar = parse_sidecar_section(raw).unwrap();
        assert!(!sidecar.git_sidecar_enabled);
    }
}
