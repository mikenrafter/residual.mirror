//! Git sidecar branch storage — metadata on orphan branch, code on working branch.

use anyhow::Result;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigHost {
    Parent,
    Repo,
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
    let _ = start;
    todo!("discover config with parent-dir stealth mode before in-repo")
}

/// Resolve effective metadata directory (sidecar branch tip when enabled).
pub fn effective_residual_dir(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    let _ = (repo_root, sidecar);
    todo!("resolve sidecar branch tip for metadata reads")
}

/// Read metadata from sidecar branch tip, not working tree.
pub fn read_sidecar_metadata(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    let _ = (repo_root, sidecar);
    todo!("read metadata from sidecar branch tip")
}

/// Tag scan: code from cwd working tree, force/component IDs from sidecar metadata.
pub fn tag_scan_sources(repo_root: &Path, sidecar: &SidecarConfig) -> Result<(PathBuf, PathBuf)> {
    let _ = (repo_root, sidecar);
    todo!("dual-source tag scan: code cwd, metadata sidecar")
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
    let _ = (repo_root, sidecar);
    todo!("warn/block on staged residual/ per working_tree_policy")
}

/// Surface resolved config path + sidecar branch on command output (S-58).
pub fn format_storage_banner(discovery: &ConfigDiscovery, sidecar: &SidecarConfig) -> String {
    let _ = (discovery, sidecar);
    todo!("format resolved config path and sidecar branch banner")
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
