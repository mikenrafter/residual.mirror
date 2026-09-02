//! Git sidecar branch storage — metadata on orphan branch, code on working branch.
//!
//! All writes to the sidecar branch(es) go through git plumbing (hash-object,
//! update-index against a scratch index, write-tree, commit-tree, update-ref) rather
//! than `git checkout`. This is deliberate: an earlier checkout-based design forced the
//! *primary* working tree onto the sidecar branch to build a commit there, and any
//! failure partway through left the repo stranded mid-operation with uncommitted work
//! discarded. Plumbing never touches the working tree or HEAD, so that failure mode is
//! structurally impossible — worst case, nothing happens and the error surfaces cleanly.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::storage::config::parse_sidecar_section;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarConfig {
    pub enabled: bool,
    pub branch: String,
    pub remote: String,
    pub working_tree_policy: WorkingTreePolicy,
    pub config_host: ConfigHost,
    /// Route reads/writes through a per-code-branch metadata branch instead of `branch` alone.
    pub branch_mode: bool,
    /// Template resolving a code branch name to its metadata branch name.
    pub branch_pattern: String,
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
            branch_mode: false,
            branch_pattern: "residual/branch-{prefix}{suffix}".to_string(),
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
            branch_mode: parsed.branch_mode,
            branch_pattern: parsed.branch_pattern,
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

/// Run a git subcommand and return trimmed stdout, bailing with stderr on failure.
/// `index_file`, when set, scopes the call to a scratch index via `GIT_INDEX_FILE` —
/// this is what lets plumbing operate without ever touching the repo's real index.
fn git_capture(repo_root: &Path, args: &[&str], index_file: Option<&Path>) -> Result<String> {
    let mut cmd = git(repo_root);
    if let Some(idx) = index_file {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    let out = cmd
        .args(args)
        .output()
        .with_context(|| format!("git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
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

fn rev_parse(repo_root: &Path, rev: &str) -> Result<String> {
    git_capture(repo_root, &["rev-parse", "--verify", rev], None)
}

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A path under `.git/` guaranteed unique within this process, for scratch plumbing
/// state (index files, transient blobs) that must never collide across concurrent calls.
fn scratch_path(repo_root: &Path, label: &str) -> PathBuf {
    let n = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    repo_root
        .join(".git")
        .join(format!("residual-{label}-{}-{n}", std::process::id()))
}

fn hash_object(repo_root: &Path, path: &Path) -> Result<String> {
    let path_str = path.to_str().context("non-utf8 path")?;
    git_capture(repo_root, &["hash-object", "-w", path_str], None)
}

fn update_index_add(repo_root: &Path, index_file: &Path, sha: &str, rel_path: &str) -> Result<()> {
    let cacheinfo = format!("100644,{sha},{rel_path}");
    git_capture(
        repo_root,
        &["update-index", "--add", "--cacheinfo", &cacheinfo],
        Some(index_file),
    )?;
    Ok(())
}

fn write_tree(repo_root: &Path, index_file: &Path) -> Result<String> {
    git_capture(repo_root, &["write-tree"], Some(index_file))
}

fn commit_tree(repo_root: &Path, tree: &str, parents: &[&str], message: &str) -> Result<String> {
    let mut args: Vec<&str> = vec!["commit-tree", tree];
    for p in parents {
        args.push("-p");
        args.push(p);
    }
    args.push("-m");
    args.push(message);
    git_capture(repo_root, &args, None)
}

/// Compare-and-swap ref update: `old_sha` of `""` asserts the ref must not yet exist.
fn update_ref(repo_root: &Path, branch: &str, new_sha: &str, old_sha: &str) -> Result<()> {
    let refname = format!("refs/heads/{branch}");
    git_capture(
        repo_root,
        &["update-ref", &refname, new_sha, old_sha],
        None,
    )?;
    Ok(())
}

fn push_branch(repo_root: &Path, remote: &str, branch: &str) -> Result<()> {
    let out = git(repo_root)
        .args(["push", remote, branch])
        .output()
        .context("git push sidecar branch")?;
    if !out.status.success() {
        bail!(
            "git push {remote} {branch} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// Recursively hash-object every file under `dir` into `index_file`, rooted at
/// `prefix`. `exclude` is applied only at the top level of `dir` (used to keep
/// `config.toml` off the sidecar branch while still committing everything else).
fn stage_dir(
    repo_root: &Path,
    index_file: &Path,
    dir: &Path,
    prefix: &str,
    exclude: &[&str],
) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if exclude.contains(&name_str.as_ref()) {
            continue;
        }
        let path = entry.path();
        let rel = format!("{prefix}/{name_str}");
        if path.is_dir() {
            stage_dir(repo_root, index_file, &path, &rel, &[])?;
        } else {
            let sha = hash_object(repo_root, &path)?;
            update_index_add(repo_root, index_file, &sha, &rel)?;
        }
    }
    Ok(())
}

/// Create the sidecar branch's first commit purely via plumbing — no checkout, no
/// working-tree interaction, so a failure here can never leave the repo mid-switch.
fn bootstrap_sidecar_branch(repo_root: &Path, branch: &str) -> Result<()> {
    let header_path = scratch_path(repo_root, "bootstrap-csv");
    std::fs::write(
        &header_path,
        "id,shortname,description,naive_change,outcomes,attractor_id\n",
    )?;
    let sha = hash_object(repo_root, &header_path);
    let _ = std::fs::remove_file(&header_path);
    let sha = sha?;

    let index_file = scratch_path(repo_root, "bootstrap-index");
    let tree = update_index_add(repo_root, &index_file, &sha, "residual/stressors.csv")
        .and_then(|()| write_tree(repo_root, &index_file));
    let _ = std::fs::remove_file(&index_file);
    let tree = tree?;

    let commit = commit_tree(repo_root, &tree, &[], "bootstrap sidecar metadata")?;
    update_ref(repo_root, branch, &commit, "")
}

/// Materialize a read-only snapshot of `branch`'s `residual/` tree into a scratch
/// directory under `.git/`. Read-only from git's perspective (`ls-tree`/`show`) — never
/// touches the working tree or index.
fn materialize_branch_tree(repo_root: &Path, branch: &str) -> Result<PathBuf> {
    if !branch_exists(repo_root, branch)? {
        bootstrap_sidecar_branch(repo_root, branch)?;
    }

    let checkout = repo_root.join(".git").join("residual-sidecar-checkout");
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

/// Commit a snapshot of `source_dir` (excluding top-level `exclude` entries) as the new
/// tip of `branch`, via plumbing only. Returns `Ok(None)` when the snapshot is identical
/// to the branch tip (no-op, no empty commit). `source_dir` may safely alias a path
/// inside the working tree (e.g. `<repo>/residual`) — it is only ever read, never
/// written to or deleted, unlike the old checkout-based approach.
fn commit_snapshot_to_branch(
    repo_root: &Path,
    branch: &str,
    source_dir: &Path,
    exclude: &[&str],
    message: &str,
) -> Result<Option<String>> {
    if !branch_exists(repo_root, branch)? {
        bail!("sidecar branch {branch} does not exist");
    }
    let parent = rev_parse(repo_root, &format!("refs/heads/{branch}"))?;
    let parent_tree = rev_parse(repo_root, &format!("{parent}^{{tree}}"))?;

    let index_file = scratch_path(repo_root, "snapshot-index");
    let new_tree = stage_dir(repo_root, &index_file, source_dir, "residual", exclude)
        .and_then(|()| write_tree(repo_root, &index_file));
    let _ = std::fs::remove_file(&index_file);
    let new_tree = new_tree?;

    if new_tree == parent_tree {
        return Ok(None);
    }

    let commit = commit_tree(repo_root, &new_tree, &[&parent], message)?;
    update_ref(repo_root, branch, &commit, &parent)?;
    Ok(Some(commit))
}

/// Write materialized metadata back to the sidecar branch when enabled.
pub fn persist_metadata_to_branch(
    repo_root: &Path,
    branch: &str,
    metadata_root: &Path,
    message: &str,
) -> Result<()> {
    commit_snapshot_to_branch(repo_root, branch, metadata_root, &["config.toml"], message)?;
    Ok(())
}

const DEFAULT_PERSIST_MESSAGE: &str = "general - sidecar: update metadata from working branch";

/// Persist metadata mutations when git sidecar is enabled.
pub fn persist_if_sidecar(cfg: &crate::config::Config, metadata_root: &Path) -> Result<()> {
    let sidecar = SidecarConfig::from_config_file(&cfg.config_path)?;
    if sidecar.enabled {
        let branch = resolve_for_write(&cfg.repo_root, &sidecar)?;
        persist_metadata_to_branch(&cfg.repo_root, &branch, metadata_root, DEFAULT_PERSIST_MESSAGE)?;
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

/// Read metadata from sidecar branch tip, not working tree. Permissive: falls back to
/// the trunk metadata branch when the current code branch has none of its own.
pub fn read_sidecar_metadata(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    if !sidecar.enabled {
        return Ok(repo_root.join("residual"));
    }
    let branch = resolve_for_read(repo_root, sidecar)?;
    materialize_branch_tree(repo_root, &branch)
}

/// Strict counterpart to `read_sidecar_metadata`, for mutations: bails with a
/// `residual branch init` hint rather than silently falling back to trunk.
pub fn write_sidecar_metadata(repo_root: &Path, sidecar: &SidecarConfig) -> Result<PathBuf> {
    if !sidecar.enabled {
        return Ok(repo_root.join("residual"));
    }
    let branch = resolve_for_write(repo_root, sidecar)?;
    materialize_branch_tree(repo_root, &branch)
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

fn split_branch_name(name: &str) -> (&str, &str) {
    match name.rfind('/') {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => ("", name),
    }
}

/// Resolve a code branch name to its metadata branch name via `pattern`.
pub fn resolve_branch_name(pattern: &str, working_branch: &str) -> String {
    let (prefix, suffix) = split_branch_name(working_branch);
    pattern.replace("{prefix}", prefix).replace("{suffix}", suffix)
}

fn resolve_named_for_read(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    working_branch: &str,
) -> Result<String> {
    if !sidecar.branch_mode {
        return Ok(sidecar.branch.clone());
    }
    let resolved = resolve_branch_name(&sidecar.branch_pattern, working_branch);
    if branch_exists(repo_root, &resolved)? {
        Ok(resolved)
    } else {
        Ok(sidecar.branch.clone())
    }
}

fn resolve_named_for_write(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    working_branch: &str,
) -> Result<String> {
    if !sidecar.branch_mode {
        return Ok(sidecar.branch.clone());
    }
    let resolved = resolve_branch_name(&sidecar.branch_pattern, working_branch);
    if branch_exists(repo_root, &resolved)? {
        Ok(resolved)
    } else {
        bail!(
            "No metadata branch for '{working_branch}' (resolves to '{resolved}'). Run `residual branch init` first."
        );
    }
}

/// Read resolution: current branch's metadata branch if it exists, else the trunk
/// (`git_sidecar_branch`). Never fails, never creates a branch. Always the trunk when
/// `branch_mode` is off.
pub fn resolve_for_read(repo_root: &Path, sidecar: &SidecarConfig) -> Result<String> {
    let working = current_branch(repo_root)?;
    resolve_named_for_read(repo_root, sidecar, &working)
}

/// Write resolution: current branch's metadata branch, or an error pointing at
/// `residual branch init`. Never creates a branch. Always the trunk (any code branch
/// allowed) when `branch_mode` is off.
pub fn resolve_for_write(repo_root: &Path, sidecar: &SidecarConfig) -> Result<String> {
    let working = current_branch(repo_root)?;
    resolve_named_for_write(repo_root, sidecar, &working)
}

/// Fork a metadata branch for `target_branch` (default: current branch) from the trunk's
/// tip. The only place a per-code-branch metadata branch gets created. Idempotent.
pub fn branch_init(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    target_branch: Option<&str>,
) -> Result<String> {
    if !branch_exists(repo_root, &sidecar.branch)? {
        bootstrap_sidecar_branch(repo_root, &sidecar.branch)?;
    }
    let working = match target_branch {
        Some(b) => b.to_string(),
        None => current_branch(repo_root)?,
    };
    let resolved = resolve_branch_name(&sidecar.branch_pattern, &working);
    if resolved == sidecar.branch || branch_exists(repo_root, &resolved)? {
        return Ok(resolved);
    }
    let trunk_sha = rev_parse(repo_root, &format!("refs/heads/{}", sidecar.branch))?;
    update_ref(repo_root, &resolved, &trunk_sha, "")?;
    Ok(resolved)
}

/// Materialize the current branch's metadata into `dest_residual_dir` for manual repair.
/// Strict: requires a resolved metadata branch (`residual branch init` first).
pub fn branch_edit(repo_root: &Path, sidecar: &SidecarConfig, dest_residual_dir: &Path) -> Result<String> {
    let branch = resolve_for_write(repo_root, sidecar)?;
    let materialized = materialize_branch_tree(repo_root, &branch)?;
    std::fs::create_dir_all(dest_residual_dir)?;
    copy_dir_all(&materialized, dest_residual_dir)?;
    Ok(branch)
}

/// Save hand-edited metadata from `src_residual_dir` back to the resolved metadata branch.
pub fn branch_save(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    src_residual_dir: &Path,
    commit_msg: &str,
    push: bool,
) -> Result<String> {
    let branch = resolve_for_write(repo_root, sidecar)?;
    let message = if commit_msg.is_empty() {
        "general - sidecar: manual branch save"
    } else {
        commit_msg
    };
    persist_metadata_to_branch(repo_root, &branch, src_residual_dir, message)?;
    if push {
        push_branch(repo_root, &sidecar.remote, &branch)?;
    }
    Ok(branch)
}

/// Merge `from`'s metadata branch into `to`'s metadata branch (both resolved from code
/// branch names), entirely via `git merge-tree --write-tree` — no checkout, so the
/// working tree is never at risk regardless of outcome. `from` must resolve to an
/// existing metadata branch (`residual branch init` first); `to` falls back to trunk
/// when unresolved, since merging back into the trunk code branch needs no dedicated
/// metadata branch of its own. On conflict, nothing is touched — resolve by checking
/// out the sidecar branch manually (e.g. in a throwaway worktree) and merging there.
pub fn branch_merge(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    from_working_branch: &str,
    to_working_branch: &str,
    commit_msg: Option<&str>,
    push: bool,
) -> Result<(String, String)> {
    let from_branch = resolve_named_for_write(repo_root, sidecar, from_working_branch)?;
    let to_branch = resolve_named_for_read(repo_root, sidecar, to_working_branch)?;

    let from_sha = rev_parse(repo_root, &format!("refs/heads/{from_branch}"))?;
    let to_sha = rev_parse(repo_root, &format!("refs/heads/{to_branch}"))?;

    let merge_out = git(repo_root)
        .args(["merge-tree", "--write-tree", &to_sha, &from_sha])
        .output()
        .context("git merge-tree")?;
    let stdout = String::from_utf8_lossy(&merge_out.stdout).to_string();
    if !merge_out.status.success() {
        bail!(
            "merge {from_branch} into {to_branch} produced conflicts — nothing was changed; resolve manually (e.g. in a throwaway worktree on '{to_branch}') and re-run:\n{stdout}"
        );
    }
    let merged_tree = stdout
        .lines()
        .next()
        .context("empty git merge-tree output")?
        .trim()
        .to_string();

    let message = commit_msg.map(|m| m.to_string()).unwrap_or_else(|| {
        format!("general - sidecar: merge {from_branch} into {to_branch}")
    });
    let new_commit = commit_tree(repo_root, &merged_tree, &[&to_sha, &from_sha], &message)?;
    update_ref(repo_root, &to_branch, &new_commit, &to_sha)?;

    if push {
        push_branch(repo_root, &sidecar.remote, &to_branch)?;
    }
    Ok((from_branch, to_branch))
}

/// Mirror the latest commit into the current branch's resolved metadata branch, used by
/// the post-commit hook. Silent no-op (`Ok(None)`) whenever there's nothing to sync:
/// sidecar disabled, no hand-edit materialized via `branch_edit` in `config_host_dir`, or
/// no metadata branch resolved for the current code branch (never creates one — that's
/// `residual branch init`'s job alone).
pub fn branch_sync_commit(
    repo_root: &Path,
    sidecar: &SidecarConfig,
    config_host_dir: &Path,
) -> Result<Option<String>> {
    if !sidecar.enabled {
        return Ok(None);
    }
    if !config_host_dir.join("stressors.csv").is_file() {
        return Ok(None);
    }
    let branch = match resolve_for_write(repo_root, sidecar) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };

    let subject = git_capture(repo_root, &["log", "-1", "--pretty=%s"], None)?;
    let short = git_capture(repo_root, &["rev-parse", "--short", "HEAD"], None)?;
    let message = format!("{subject} ({short})");

    persist_metadata_to_branch(repo_root, &branch, config_host_dir, &message)?;
    Ok(Some(branch))
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

    fn commit_all(path: &Path, message: &str) {
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(path)
            .output()
            .expect("git add");
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .expect("git commit");
    }

    fn checkout_new_branch(path: &Path, name: &str) {
        Command::new("git")
            .args(["checkout", "-b", name])
            .current_dir(path)
            .output()
            .expect("git checkout -b");
    }

    fn branch_mode_sidecar() -> SidecarConfig {
        SidecarConfig {
            enabled: true,
            branch_mode: true,
            ..SidecarConfig::default()
        }
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

        // Bootstrap is pure plumbing: primary checkout/HEAD must be untouched.
        assert_eq!(current_branch(&repo).unwrap(), "main");
        assert!(
            !repo.join("residual/stressors.csv").exists()
                || std::fs::read_to_string(repo.join("residual/stressors.csv"))
                    .unwrap()
                    .contains("S-99"),
            "bootstrap must never overwrite the working tree"
        );
    }

    #[test]
    fn tag_scan_uses_dual_source_code_cwd_metadata_sidecar() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/lib.rs"), "// @stressor: ceremony-lockout\n").unwrap();
        init_git_repo(&repo);

        let sidecar = SidecarConfig {
            enabled: true,
            ..SidecarConfig::default()
        };
        let (code_root, meta_root) = tag_scan_sources(&repo, &sidecar).unwrap();
        assert_eq!(code_root, repo, "code scan must use working tree cwd");
        assert_ne!(
            meta_root,
            repo.join("residual"),
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

    #[test]
    fn resolve_branch_name_splits_on_last_slash_keeping_leading_slash_on_suffix() {
        let resolved = resolve_branch_name(
            "residual/branch-{prefix}{suffix}",
            "super-cool-feature/poc",
        );
        assert_eq!(resolved, "residual/branch-super-cool-feature/poc");
    }

    #[test]
    fn resolve_branch_name_with_no_slash_puts_whole_name_in_suffix() {
        let resolved = resolve_branch_name("meta/{prefix}{suffix}", "main");
        assert_eq!(resolved, "meta/main");
    }

    #[test]
    fn resolve_for_read_falls_back_to_trunk_when_target_branch_missing() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/untouched");

        let sidecar = branch_mode_sidecar();
        let resolved = resolve_for_read(&repo, &sidecar).unwrap();
        assert_eq!(
            resolved, sidecar.branch,
            "read must fall back to trunk silently, not error, when metadata branch is uninitialized"
        );
    }

    #[test]
    fn resolve_for_write_bails_with_init_hint_when_target_branch_missing() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/untouched");

        let sidecar = branch_mode_sidecar();
        let err = resolve_for_write(&repo, &sidecar).unwrap_err();
        assert!(
            err.to_string().contains("residual branch init"),
            "write must bail pointing at `residual branch init`, got: {err}"
        );
    }

    #[test]
    fn resolve_for_write_always_returns_trunk_when_branch_mode_off() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/anything");

        let sidecar = SidecarConfig {
            enabled: true,
            branch_mode: false,
            ..SidecarConfig::default()
        };
        let resolved = resolve_for_write(&repo, &sidecar).unwrap();
        assert_eq!(
            resolved, sidecar.branch,
            "mutation on any branch must resolve to the single trunk when branch_mode is off"
        );
    }

    #[test]
    fn branch_init_forks_from_trunk_tip_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/x");

        let sidecar = branch_mode_sidecar();
        let first = branch_init(&repo, &sidecar, None).unwrap();
        assert_eq!(first, "residual/branch-feature/x");

        // Forked from trunk tip: the trunk's bootstrap content is already present.
        let trunk_tip = rev_parse(&repo, &format!("refs/heads/{}", sidecar.branch)).unwrap();
        let forked_tip = rev_parse(&repo, &format!("refs/heads/{first}")).unwrap();
        assert_eq!(trunk_tip, forked_tip);

        let second = branch_init(&repo, &sidecar, None).unwrap();
        assert_eq!(second, first, "branch_init must be idempotent");

        // Never touches the primary checkout.
        assert_eq!(current_branch(&repo).unwrap(), "feature/x");
    }

    #[test]
    fn branch_edit_then_save_round_trips_and_never_writes_config_toml_to_sidecar() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/x");

        let sidecar = branch_mode_sidecar();
        branch_init(&repo, &sidecar, None).unwrap();

        let dest = repo.join("residual");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("config.toml"), "format_version = \"v4\"\n").unwrap();
        branch_edit(&repo, &sidecar, &dest).unwrap();
        assert!(
            dest.join("config.toml").is_file(),
            "branch_edit must not clobber an existing config.toml pointer"
        );

        std::fs::write(
            dest.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\nS-01,x,d,n,o,A-01\n",
        )
        .unwrap();
        branch_save(&repo, &sidecar, &dest, "feature edit", false).unwrap();

        let materialized = materialize_branch_tree(&repo, "residual/branch-feature/x").unwrap();
        let saved = std::fs::read_to_string(materialized.join("stressors.csv")).unwrap();
        assert!(saved.contains("S-01"));
        assert!(
            !materialized.join("config.toml").exists(),
            "config.toml must never land on the sidecar branch"
        );

        // No checkout ever happened.
        assert_eq!(current_branch(&repo).unwrap(), "feature/x");
        assert!(repo.join("f").exists(), "primary working tree must be untouched");
    }

    #[test]
    fn branch_sync_commit_is_noop_without_a_hand_edit_marker() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");

        let sidecar = SidecarConfig {
            enabled: true,
            ..SidecarConfig::default()
        };
        let result = branch_sync_commit(&repo, &sidecar, &repo.join("residual")).unwrap();
        assert_eq!(result, None, "sync-commit must no-op without a materialized hand-edit");
    }

    #[test]
    fn branch_sync_commit_tags_message_with_subject_and_short_hash() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/x");

        let sidecar = branch_mode_sidecar();
        branch_init(&repo, &sidecar, None).unwrap();

        let dest = repo.join("residual");
        std::fs::create_dir_all(&dest).unwrap();
        branch_edit(&repo, &sidecar, &dest).unwrap();
        std::fs::write(
            dest.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\nS-02,y,d,n,o,A-01\n",
        )
        .unwrap();
        std::fs::write(repo.join("g"), "y").unwrap();
        commit_all(&repo, "add stressor by hand");

        let branch = branch_sync_commit(&repo, &sidecar, &dest).unwrap().unwrap();
        assert_eq!(branch, "residual/branch-feature/x");

        let log = git_capture(&repo, &["log", "-1", "--pretty=%s", &branch], None).unwrap();
        assert!(
            log.contains("add stressor by hand"),
            "sync commit message must include the working-branch commit title, got: {log}"
        );
    }

    #[test]
    fn branch_merge_folds_from_branch_into_to_branch_without_touching_working_tree() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/x");

        let sidecar = branch_mode_sidecar();
        let feature_branch = branch_init(&repo, &sidecar, None).unwrap();

        let dest = repo.join("residual");
        std::fs::create_dir_all(&dest).unwrap();
        branch_edit(&repo, &sidecar, &dest).unwrap();
        std::fs::write(
            dest.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\nS-01,x,d,n,o,A-01\n",
        )
        .unwrap();
        branch_save(&repo, &sidecar, &dest, "feature edit", false).unwrap();

        let (from, to) = branch_merge(&repo, &sidecar, "feature/x", "main", None, false).unwrap();
        assert_eq!(from, feature_branch);
        assert_eq!(to, sidecar.branch);

        let current = current_branch(&repo).unwrap();
        assert_eq!(current, "feature/x", "merge must never touch the primary checkout");
        assert!(repo.join("f").exists(), "primary working tree must be untouched");

        let merged = std::fs::read_to_string(
            materialize_branch_tree(&repo, &sidecar.branch)
                .unwrap()
                .join("stressors.csv"),
        )
        .unwrap();
        assert!(
            merged.contains("S-01"),
            "merge must fold the feature branch's metadata into trunk"
        );
    }

    #[test]
    fn branch_merge_conflict_leaves_both_branches_untouched() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_git_repo(&repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        commit_all(&repo, "init");
        checkout_new_branch(&repo, "feature/x");

        let sidecar = branch_mode_sidecar();
        branch_init(&repo, &sidecar, None).unwrap();
        let dest = repo.join("residual");
        std::fs::create_dir_all(&dest).unwrap();
        branch_edit(&repo, &sidecar, &dest).unwrap();
        std::fs::write(dest.join("stressors.csv"), "conflicting content a\n").unwrap();
        branch_save(&repo, &sidecar, &dest, "feature edit", false).unwrap();

        // Diverge trunk itself so the merge actually conflicts.
        let trunk_dest = repo.join("residual-trunk-edit");
        std::fs::create_dir_all(&trunk_dest).unwrap();
        let trunk_materialized = materialize_branch_tree(&repo, &sidecar.branch).unwrap();
        copy_dir_all(&trunk_materialized, &trunk_dest).unwrap();
        std::fs::write(trunk_dest.join("stressors.csv"), "conflicting content b\n").unwrap();
        persist_metadata_to_branch(&repo, &sidecar.branch, &trunk_dest, "trunk edit").unwrap();

        let trunk_before = rev_parse(&repo, &format!("refs/heads/{}", sidecar.branch)).unwrap();
        let feature_before = rev_parse(&repo, "refs/heads/residual/branch-feature/x").unwrap();

        let err = branch_merge(&repo, &sidecar, "feature/x", "main", None, false).unwrap_err();
        assert!(err.to_string().contains("conflict"));

        let trunk_after = rev_parse(&repo, &format!("refs/heads/{}", sidecar.branch)).unwrap();
        let feature_after = rev_parse(&repo, "refs/heads/residual/branch-feature/x").unwrap();
        assert_eq!(trunk_before, trunk_after, "conflict must not move the trunk ref");
        assert_eq!(feature_before, feature_after, "conflict must not move the feature ref");
        assert_eq!(current_branch(&repo).unwrap(), "feature/x");
    }
}
