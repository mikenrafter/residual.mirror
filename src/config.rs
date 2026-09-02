use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub validation: ValidationConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    /// In-repo residual/ hosting config.toml (config pointer when sidecar is enabled).
    /// Use [`crate::storage::metadata_dir`] for ledger reads when sidecar may be enabled.
    #[serde(skip)]
    pub residual_dir: PathBuf,
    /// In-repo residual/ hosting config.toml (config pointer when sidecar is enabled).
    #[serde(skip)]
    pub config_host_dir: PathBuf,
    /// Resolved config.toml path used for policy and sidecar settings.
    #[serde(skip)]
    pub config_path: PathBuf,
    /// Repository root for code scans and git sidecar operations.
    #[serde(skip)]
    pub repo_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default = "default_strict")]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    #[serde(default = "default_token_warn")]
    pub token_warn: usize,
}

fn default_strict() -> bool { true }
fn default_token_warn() -> usize { 1000 }

impl Default for ValidationConfig {
    fn default() -> Self { Self { strict: default_strict() } }
}

impl Default for SkillsConfig {
    fn default() -> Self { Self { token_warn: default_token_warn() } }
}

impl Default for Config {
    fn default() -> Self {
        let residual_dir = PathBuf::from("residual");
        Self {
            validation: ValidationConfig::default(),
            skills: SkillsConfig::default(),
            config_path: residual_dir.join("config.toml"),
            config_host_dir: residual_dir.clone(),
            repo_root: PathBuf::from("."),
            residual_dir,
        }
    }
}

impl Config {
    /// Test helper: inline metadata (no git sidecar resolution).
    pub fn for_test_residual_dir(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        Self {
            validation: ValidationConfig::default(),
            skills: SkillsConfig::default(),
            residual_dir: dir.clone(),
            config_host_dir: dir.clone(),
            config_path: dir.join("config.toml"),
            repo_root: dir
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(".")),
        }
    }
}

pub fn load() -> Result<Config> {
    let repo_root = std::env::current_dir().context("get current dir")?;
    let discovery = crate::storage::git_sidecar::discover_config(&repo_root)?;
    let config_path = discovery.config_path.clone();

    let mut cfg = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        parse_any(&raw).with_context(|| format!("parse {}", config_path.display()))?
    } else {
        Config::default()
    };

    cfg.repo_root = repo_root.clone();
    cfg.config_path = config_path;
    cfg.config_host_dir = discovery.residual_dir.clone();
    // Keep residual_dir as the config-host path; sidecar materialization is lazy via storage::metadata_dir.
    cfg.residual_dir = cfg.config_host_dir.clone();
    Ok(cfg)
}

pub fn print(cfg: &Config) -> Result<()> {
    println!("config_host_dir = {}", cfg.config_host_dir.display());
    match crate::storage::metadata_dir(cfg) {
        Ok(dir) => println!("metadata_dir = {}", dir.display()),
        Err(err) => println!("metadata_dir = <unresolved: {err}>"),
    }
    println!("validation.strict = {}", cfg.validation.strict);
    println!("skills.token_warn = {}", cfg.skills.token_warn);
    Ok(())
}

pub fn residual_dir(cfg: &Config) -> &Path {
    &cfg.residual_dir
}

#[derive(Debug, Deserialize)]
struct V3Shim {
    #[serde(default)]
    verification: Option<V3VerificationSection>,
    #[serde(default)]
    validation: Option<ValidationConfig>,
    #[serde(default)]
    skills: Option<SkillsConfig>,
}

#[derive(Debug, Deserialize)]
struct V3VerificationSection {
    #[serde(default = "default_strict")]
    super_strict: bool,
    #[serde(default = "default_token_warn")]
    token_warn: usize,
}

fn parse_any(raw: &str) -> Result<Config> {
    // Storage TOML ([verification] / [storage] / format_version v3 or v4). Policy keys map onto
    // the legacy Config fields so the rest of the binary stays stable.
    if raw.contains("format_version") || raw.contains("[verification]") || raw.contains("[storage]")
    {
        let shim: V3Shim = toml::from_str(raw)?;
        let (strict, token_warn) = if let Some(v) = shim.verification {
            (v.super_strict, v.token_warn)
        } else {
            (
                shim.validation.map(|v| v.strict).unwrap_or_else(default_strict),
                shim.skills.map(|s| s.token_warn).unwrap_or_else(default_token_warn),
            )
        };
        return Ok(Config {
            validation: ValidationConfig { strict },
            skills: SkillsConfig { token_warn },
            ..Config::default()
        });
    }
    Ok(toml::from_str::<Config>(raw)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper: parse a config.toml string with a given residual_dir.
    fn parse_with_dir(toml_str: &str, dir: &Path) -> Config {
        let mut cfg = if toml_str.trim().is_empty() {
            Config::for_test_residual_dir(dir)
        } else if toml_str.contains("format_version")
            || toml_str.contains("[verification]")
            || toml_str.contains("[storage]")
        {
            parse_any(toml_str).expect("failed to parse config toml")
        } else {
            toml::from_str(toml_str).expect("failed to parse config toml")
        };
        cfg.residual_dir = dir.to_path_buf();
        cfg.config_host_dir = dir.to_path_buf();
        cfg.config_path = dir.join("config.toml");
        cfg.repo_root = dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        cfg
    }

    #[test]
    fn defaults_strict_true() {
        let cfg = Config::default();
        assert!(cfg.validation.strict, "default strict should be true");
    }

    #[test]
    fn defaults_token_warn_1000() {
        let cfg = Config::default();
        assert_eq!(cfg.skills.token_warn, 1000);
    }

    #[test]
    fn load_no_config_file_returns_defaults() {
        let dir = tempdir().unwrap();
        // No config.toml in dir — simulate by parsing empty toml
        let cfg = parse_with_dir("", dir.path());
        assert!(cfg.validation.strict, "expected strict=true when no config");
        assert_eq!(cfg.skills.token_warn, 1000);
    }

    #[test]
    fn load_config_with_strict_false() {
        let dir = tempdir().unwrap();
        let toml_str = "[validation]\nstrict = false\n";
        let cfg = parse_with_dir(toml_str, dir.path());
        assert!(!cfg.validation.strict, "expected strict=false when set in config");
    }

    #[test]
    fn load_config_with_custom_token_warn() {
        let dir = tempdir().unwrap();
        let toml_str = "[skills]\ntoken_warn = 500\n";
        let cfg = parse_with_dir(toml_str, dir.path());
        assert_eq!(cfg.skills.token_warn, 500);
    }

    #[test]
    fn load_succeeds_without_git_repo_when_sidecar_config_present() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("proj");
        std::fs::create_dir_all(project.join("residual")).unwrap();
        std::fs::write(
            project.join("residual/config.toml"),
            r#"
format_version = "v4"
[storage]
git_sidecar_enabled = true
git_sidecar_branch = "residual/metadata"
config_host = "repo"
"#,
        )
        .unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project).unwrap();
        let cfg = super::load().expect("load must not touch git during config discovery");
        assert_eq!(cfg.config_host_dir, project.join("residual"));
        std::env::set_current_dir(prev).unwrap();
    }

    #[test]
    fn discover_config_prefers_parent_stealth_over_in_repo() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("myproject");
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(repo.join("residual/config.toml"), "[validation]\nstrict = true\n").unwrap();
        let parent_residual = dir.path().join("residual");
        std::fs::create_dir_all(&parent_residual).unwrap();
        std::fs::write(
            parent_residual.join("config.toml"),
            "format_version = \"v4\"\n[storage]\nconfig_host = \"parent\"\n",
        )
        .unwrap();

        let discovery = crate::storage::git_sidecar::discover_config(&repo).unwrap();
        assert_eq!(
            discovery.source,
            crate::storage::git_sidecar::ConfigSource::ParentStealth
        );
    }
}
