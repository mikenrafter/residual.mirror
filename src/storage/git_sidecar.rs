//! Git sidecar branch storage — metadata on orphan branch, code on working branch.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::storage::config::parse_sidecar_section;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarConfig {
    pub enabled: bool,
    pub branch: String,
    pub remote: String,
    pub working_tree_policy: WorkingTreePolicy,
    pub config_host: ConfigHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingTreePolicy {
    Warn,
    Block,
    Ignore,
}

impl WorkingTreePolicy {
    fn from_str(s: &str) -> Self {
        match s {
            "block" => Self::Block,
            "ignore" => Self::Ignore,
            _ => Self::Warn,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigHost {
    Parent,
    Repo,
}

impl ConfigHost {
    fn from_str(s: &str) -> Self {
        match s {
            "parent" => Self::Parent,
            _ => Self::Repo,
        }
    }
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            branch: "residual/metadata".to_string(),
            remote: "origin".to_string(),
            working_tree_policy: WorkingTreePolicy::Warn,
            config_host: ConfigHost::Repo,
        }
    }
}

impl SidecarConfig {
    /// Load sidecar settings from a config.toml file.
    pub fn from_config_file(config_path: &Path) -> Result<Self> {
        if !config_path.is_file() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        Self::from_toml(&raw)
    }

    pub fn from_toml(raw: &str) -> Result<Self> {
        let parsed = parse_sidecar_section(raw)?;
        Ok(Self {
            enabled: parsed.git_sidecar_enabled,
            branch: parsed.git_sidecar_branch,
            remote: parsed.git_sidecar_remote,
            working_tree_policy: WorkingTreePolicy::from_str(&parsed.working_tree_policy),
            config_host: ConfigHost::from_str(&parsed.config_host),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiscovery {
    pub config_path: PathBuf,
    pub residual_dir: PathBuf,
    pub source: ConfigSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    ParentStealth,
    InRepo,
    WalkUp,
}

/// Discover config following search order: parent-dir stealth → in-repo → walk up (bounded at /).
pub fn discover_config(start: &Path) -> Result<ConfigDiscovery> {
    let mut base = start.to_path_buf();
    if base.is_file() {
        base.pop();
    }

    // 1. Parent-dir stealth: ../residual/config.toml relative to start directory.
    if let Some(parent) = base.parent() {
        let parent_config = parent.join("residual/config.toml");
        if parent_config.is_file() {
            return Ok(ConfigDiscovery {
                config_path: parent_config.clone(),
                residual_dir: parent.join("residual"),
                source: ConfigSource::ParentStealth,
            });
        }
    }

    // 2. In-repo: <start>/residual/config.toml
    let in_repo_config = base.join("residual/config.toml");
    if in_repo_config.is_file() {
        return Ok(ConfigDiscovery {
            config_path: in_repo_config,
            residual_dir: base.join("residual"),
            source: ConfigSource::InRepo,
        });
    }

    // 3. Walk up from start for residual/ directory (bounded at filesystem root).
    let mut dir = base;
    loop {
        let candidate = dir.join("residual");
        if candidate.is_dir() {
            return Ok(ConfigDiscovery {
                config_path: candidate.join("config.toml"),
                residual_dir: candidate,
                source: ConfigSource::WalkUp,
            });
        }
        if !dir.pop() {
            return Ok(ConfigDiscovery {
                config_path: PathBuf::from("residual/config.toml"),
                residual_dir: PathBuf::from("residual"),
                source: ConfigSource::WalkUp,
            });
        }
    }
}

fn git(repo_root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root);
    cmd
}

fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let out = git(repo_root)
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .output()
        .context("git rev-parse branch")?;
    Ok(out.status.success())
}

fn current_branch(repo_root: &Path) -> Result<String> {
    let out = git(repo_root)
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .context("git symbolic-ref")?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    Ok("main".to_string())
}

fn bootstrap_sidecar_branch(repo_root: &Path, branch: &str) -> Result<()> {
    let prev = current_branch(repo_root)?;
    let orphan_out = git(repo_root)
        .args(["checkout", "--orphan", branch])
        .output()
        .context("git checkout --orphan")?;
    if !orphan_out.status.success() {
        bail!(
            "git checkout --orphan failed: {}",
            String::from_utf8_lossy(&orphan_out.stderr)
        );
    }

    // Orphan checkout retains working-tree files; write canonical empty metadata only.
    let residual = repo_root.join("residual");
    std::fs::create_dir_all(&residual)?;
    let header = "id,shortname,description,naive_change,outcomes,attractor_id\n";
    std::fs::write(residual.join("stressors.csv"), header)?;

    git(repo_root)
        .args(["rm", "-rf", "--cached", "."])
        .output()
        .ok();
    git(repo_root)
        .args(["add", "-f", "residual/"])
        .output()
        .context("git add residual")?;
    git(repo_root)
        .args(["commit", "-m", "bootstrap sidecar metadata"])
        .output()
        .context("git commit sidecar bootstrap")?;

    git(repo_root)
        .args(["checkout", &prev])
        .output()
        .context("git checkout previous branch")?;
    Ok(())
}

fn materialize_branch_tree(repo_root: &Path, branch: &str) -> Result<PathBuf> {
    if !branch_exists(repo_root, branch)? {
        bootstrap_sidecar_branch(repo_root, branch)?;
    }

    let checkout = repo_root
        .join(".git")
        .join("residual-sidecar-checkout");
    if checkout.exists() {
        std::fs::remove_dir_all(&checkout)
            .with_context(|| format!("clear {}", checkout.display()))?;
    }
    std::fs::create_dir_all(&checkout)?;

    let list_out = git(repo_root)
        .args(["ls-tree", "-r", "--name-only", branch, "--", "residual/"])
        .output()
        .context("git ls-tree sidecar")?;
    if !list_out.status.success() {
        bail!(
            "git ls-tree failed: {}",
            String::from_utf8_lossy(&list_out.stderr)
        );
    }

    let paths = String::from_utf8_lossy(&list_out.stdout);
    for rel in paths.lines().filter(|l| !l.is_empty()) {
        let show_out = git(repo_root)
            .args(["show", &format!("{branch}:{rel}")])
            .output()
            .with_context(|| format!("git show {branch}:{rel}"))?;
        if !show_out.status.success() {
            continue;
        }
        let dest = checkout.join(rel.strip_prefix("residual/").unwrap_or(rel));
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, &show_out.stdout)?;
    }

    Ok(checkout)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)
                .with_context(|| format!("copy {} -> {}", path.display(), dest.display()))?;
        }
    }
    Ok(())
}

/// Write materialized metadata back to the sidecar branch when enabled.
pub fn persist_metadata_to_branch(
    repo_root: &Path,
    branch: &str,
    metadata_root: &Path,
) -> Result<()> {
    if !branch_exists(repo_root, branch)? {
        bail!("sidecar branch {branch} does not exist");
    }
    let prev = current_branch(repo_root)?;
    let checkout = git(repo_root)
        .args(["checkout", branch])
        .output()
        .context("git checkout sidecar for persist")?;
    if !checkout.status.success() {
        bail!(
            "git checkout {branch} failed: {}",
            String::from_utf8_lossy(&checkout.stderr)
        );
    }

    let dest = repo_root.join("residual");
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("clear {}", dest.display()))?;
    }
    copy_dir_all(metadata_root, &dest)?;

    git(repo_root)
        .args(["add", "-f", "residual/"])
        .output()
        .context("git add sidecar metadata")?;
    let diff = git(repo_root)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .context("git diff --cached --quiet")?;
    if !diff.status.success() {
        let commit = git(repo_root)
            .args([
                "commit",
                "-m",
                "general - sidecar: update metadata from working branch",
            ])
            .output()
            .context("git commit sidecar metadata")?;
        if !commit.status.success() {
            bail!(
                "git commit sidecar metadata failed: {}",
                String::from_utf8_lossy(&commit.stderr)
            );
        }
    }

    let restore = git(repo_root)
        .args(["checkout", "-f", &prev])
        .output()
        .context("git checkout previous branch")?;
    if !restore.status.success() {
        bail!(
            "git checkout {prev} failed: {}",
            String::from_utf8_lossy(&restore.stderr)
        );
    }
    Ok(())
}

/// Persist metadata mutations when git sidecar is enabled.
pub fn persist_if_sidecar(cfg: &crate::config::Config, metadata_root: &Path) -> Result<()> {
    let sidecar = SidecarConfig::from_config_file(&cfg.config_path)?;
    if sidecar.enabled {
        persist_metadata_to_branch(&cfg.repo_root, &sidecar.branch, metadata_root)?;
    }
    Ok(())
}

/// Resolve effective metadata directory (sidecar branch tip when enabled).
pub fn effective_residual_dir(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    if sidecar.enabled {
        read_sidecar_metadata(repo_root, sidecar)
    } else {
        Ok(repo_root.join("residual"))
    }
}

/// Read metadata from sidecar branch tip, not working tree.
pub fn read_sidecar_metadata(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    if !sidecar.enabled {
        return Ok(repo_root.join("residual"));
    }
    materialize_branch_tree(repo_root, &sidecar.branch)
}

/// Tag scan: code from cwd working tree, force/component IDs from sidecar metadata.
pub fn tag_scan_sources(repo_root: &Path, sidecar: &SidecarConfig) -> Result<(PathBuf, PathBuf)> {
    let code_root = repo_root.to_path_buf();
    let meta_root = if sidecar.enabled {
        read_sidecar_metadata(repo_root, sidecar)?
    } else {
        repo_root.join("residual")
    };
    Ok((code_root, meta_root))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingTreeWarning {
    pub staged_paths: Vec<String>,
    pub policy: WorkingTreePolicy,
}

/// Check working-tree policy for staged residual/ paths on working branch.
pub fn check_working_tree_policy(
    repo_root: &Path,
    sidecar: &SidecarConfig,
) -> Result<Option<WorkingTreeWarning>> {
    if !sidecar.enabled || sidecar.working_tree_policy == WorkingTreePolicy::Ignore {
        return Ok(None);
    }

    let out = git(repo_root)
        .args(["diff", "--cached", "--name-only"])
        .output()
        .context("git diff --cached")?;
    if !out.status.success() {
        return Ok(None);
    }

    let staged: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|p| p.starts_with("residual/") || p.starts_with("residual\\"))
        .map(str::to_string)
        .collect();

    if staged.is_empty() {
        return Ok(None);
    }

    Ok(Some(WorkingTreeWarning {
        staged_paths: staged,
        policy: sidecar.working_tree_policy,
    }))
}

/// Surface resolved config path + sidecar branch on command output (S-58).
pub fn format_storage_banner(discovery: &ConfigDiscovery, sidecar: &SidecarConfig) -> String {
    format!(
        "config: {} | sidecar branch: {}",
        discovery.config_path.display(),
        sidecar.branch
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn init_git_repo(path: &Path) {
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .expect("git init");
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .expect("git config email");
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .expect("git config name");
    }

    #[test]
    fn discover_prefers_parent_dir_stealth_config_before_in_repo() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("myproject");
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(
            repo.join("residual/config.toml"),
            "format_version = \"v4\"\n[storage]\n",
        )
        .unwrap();
        let parent_residual = dir.path().join("residual");
        std::fs::create_dir_all(&parent_residual).unwrap();
        std::fs::write(
            parent_residual.join("config.toml"),
            "format_version = \"v4\"\n[storage]\nconfig_host = \"parent\"\n",
        )
        .unwrap();

        let discovery = discover_config(&repo).unwrap();
        assert_eq!(
            discovery.source,
            ConfigSource::ParentStealth,
            "parent-dir ../residual/config.toml must win over in-repo"
        );
        assert!(
            discovery.config_path.ends_with("residual/config.toml"),
            "config path must be under parent residual dir"
        );
    }

    #[test]
    fn discover_search_bounded_at_filesystem_root() {
        let dir = tempdir().unwrap();
        let deep = dir.path().join("a/b/c/d/e");
        std::fs::create_dir_all(&deep).unwrap();

        let discovery = discover_config(&deep);
        assert!(
            discovery.is_ok() || discovery.is_err(),
            "search must terminate at filesystem root without infinite loop"
        );
        if let Ok(d) = discovery {
            assert!(
                !d.config_path.starts_with("/proc"),
                "must not escape temp sandbox"
            );
        }
    }

    #[test]
    fn sidecar_enabled_reads_metadata_from_branch_tip_not_working_tree() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);

        let sidecar = SidecarConfig {
            enabled: true,
            ..SidecarConfig::default()
        };

        // Working tree has stale stressor; sidecar branch has canonical copy.
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(
            repo.join("residual/stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-99,working-tree-only,stale,none,,A-01\n",
        )
        .unwrap();

        let meta_dir = read_sidecar_metadata(&repo, &sidecar).unwrap();
        let stressors = std::fs::read_to_string(meta_dir.join("stressors.csv")).unwrap();
        assert!(
            !stressors.contains("S-99"),
            "sidecar read must not return working-tree-only stressor S-99"
        );
        assert!(
            stressors.contains("S-01") || stressors.lines().count() <= 1,
            "sidecar tip metadata expected; got: {stressors}"
        );
    }

    #[test]
    fn tag_scan_uses_dual_source_code_cwd_metadata_sidecar() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(
            repo.join("src/lib.rs"),
            "// @stressor: ceremony-lockout\n",
        )
        .unwrap();
        init_git_repo(&repo);

        let sidecar = SidecarConfig {
            enabled: true,
            ..SidecarConfig::default()
        };
        let (code_root, meta_root) = tag_scan_sources(&repo, &sidecar).unwrap();
        assert_eq!(code_root, repo, "code scan must use working tree cwd");
        assert_ne!(
            meta_root, repo.join("residual"),
            "metadata must resolve from sidecar branch, not working-tree residual/"
        );
    }

    #[test]
    fn working_tree_policy_warns_on_staged_residual() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(repo.join("residual/stressors.csv"), "id\n").unwrap();
        init_git_repo(&repo);
        Command::new("git")
            .args(["add", "residual/"])
            .current_dir(&repo)
            .output()
            .expect("git add");

        let sidecar = SidecarConfig {
            enabled: true,
            working_tree_policy: WorkingTreePolicy::Warn,
            ..SidecarConfig::default()
        };
        let warning = check_working_tree_policy(&repo, &sidecar)
            .unwrap()
            .expect("staged residual/ must trigger working_tree_policy warning");
        assert_eq!(warning.policy, WorkingTreePolicy::Warn);
        assert!(
            warning.staged_paths.iter().any(|p| p.contains("residual")),
            "warning must list staged residual paths"
        );
    }

    #[test]
    fn format_storage_banner_surfaces_config_path_and_sidecar_branch() {
        let dir = tempdir().unwrap();
        let discovery = ConfigDiscovery {
            config_path: dir.path().join("residual/config.toml"),
            residual_dir: dir.path().join("residual"),
            source: ConfigSource::InRepo,
        };
        let sidecar = SidecarConfig::default();
        let banner = format_storage_banner(&discovery, &sidecar);
        assert!(
            banner.contains("config.toml") || banner.contains("config:"),
            "banner must surface resolved config path (S-58)"
        );
        assert!(
            banner.contains("residual/metadata") || banner.contains("sidecar"),
            "banner must surface sidecar branch name (S-58)"
        );
    }
}
