//! Git hook install — pre-commit runs verification; commit-msg validates vocabulary;
//! post-commit mirrors branch-mode metadata edits into the resolved sidecar branch.
//!
//! Hooks are installed as marked blocks so residual can coexist with other tools that
//! also hook the same file (e.g. Entire CLI's post-commit checkpoint condenser):
//! foreign content is preserved and residual's block appended after it; a prior
//! residual-owned block (same marker) is replaced in place on reinstall.

use anyhow::{Context, Result};

pub fn install() -> Result<()> {
    let git_hooks_dir = find_git_hooks_dir()?;

    let pre_commit_body = r#"# residual pre-commit hook — validates residual/ data before commit
#
# Hard block: residual metadata must live on the git-sidecar branch, never on the
# working branch — except residual/config.toml, the in-repo pointer file. This check
# has no override flag by design.
LEAKED=$(git diff --cached --name-only | grep '^residual/' | grep -v '^residual/config\.toml$')
if [ -n "$LEAKED" ]; then
  echo "residual: refusing commit — residual metadata staged on the working branch (belongs on the git-sidecar branch):" >&2
  echo "$LEAKED" | sed 's/^/  /' >&2
  echo "Unstage with: git restore --staged <path>" >&2
  exit 1
fi

STAGED=$(git diff --cached --name-only | grep '^residual/')

if [ -z "$STAGED" ]; then
  STRICT=$(residual config 2>/dev/null | grep 'strict' | awk '{print $3}')
  [ "$STRICT" = "false" ] && exit 0
fi

residual verify all || exit 1
residual verify walk-reminder --staged || exit 1
"#;

    let commit_msg_body = r#"# residual commit-msg hook — lexicon/component vocabulary (warn by default)
residual verify commit-msg "$1" --staged || exit 1
"#;

    let post_commit_body = r#"# residual post-commit hook — mirrors hand-edited metadata (via `residual branch edit`)
# into the resolved metadata branch, tagging the commit with this commit's title+hash.
# Silent no-op when there's nothing to sync (most branches never touch residual data).
residual branch sync-commit >/dev/null 2>&1 || true
"#;

    let pre_commit = git_hooks_dir.join("pre-commit");
    let commit_msg = git_hooks_dir.join("commit-msg");
    let post_commit = git_hooks_dir.join("post-commit");

    write_hook(&pre_commit, "pre-commit", pre_commit_body)?;
    write_hook(&commit_msg, "commit-msg", commit_msg_body)?;
    write_hook(&post_commit, "post-commit", post_commit_body)?;

    println!("Installed pre-commit hook to {}", pre_commit.display());
    println!("Installed commit-msg hook to {} (warn by default; set commit_msg_enforce = true in residual/config.toml to block)", commit_msg.display());
    println!("Installed post-commit hook to {}", post_commit.display());
    Ok(())
}

fn find_git_hooks_dir() -> Result<std::path::PathBuf> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let mut dir = cwd.as_path();
    loop {
        let candidate = dir.join(".git/hooks");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => anyhow::bail!("could not find .git/hooks directory (not a git repository?)"),
        }
    }
}

fn marker_begin(name: &str) -> String {
    format!("# >>> residual:{name} >>>")
}

fn marker_end(name: &str) -> String {
    format!("# <<< residual:{name} <<<")
}

/// Merge `body` into `existing` hook content under a marked block for `name`: replaces
/// a prior residual block in place (reinstall), or appends after foreign content
/// (first install alongside another tool's hook), or writes a fresh file with shebang.
fn merge_hook_content(existing: Option<String>, name: &str, body: &str) -> String {
    let begin = marker_begin(name);
    let end = marker_end(name);
    let block = format!("{begin}\n{body}{end}\n");

    match existing {
        None => format!("#!/usr/bin/env bash\n{block}"),
        Some(mut content) => match (content.find(&begin), content.find(&end)) {
            (Some(b), Some(e)) => {
                let e_end = e + end.len();
                content.replace_range(b..e_end, block.trim_end());
                content
            }
            _ => {
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&block);
                content
            }
        },
    }
}

fn write_hook(path: &std::path::Path, name: &str, body: &str) -> Result<()> {
    let existing = std::fs::read_to_string(path).ok();
    let content = merge_hook_content(existing, name, body);
    std::fs::write(path, content)
        .with_context(|| format!("failed to write hook to {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn init_git_repo(path: &std::path::Path) {
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

    fn run_hook(repo: &std::path::Path, name: &str) -> std::process::Output {
        Command::new(repo.join(".git/hooks").join(name))
            .current_dir(repo)
            .output()
            .unwrap_or_else(|e| panic!("run {name} hook: {e}"))
    }

    fn install_in(repo: &std::path::Path) {
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo).unwrap();
        let installed = install();
        std::env::set_current_dir(prev).unwrap();
        installed.unwrap();
    }

    /// Contract for nix/sandbox: installed hooks must not rely on `env bash`.
    /// Prefer `#!/bin/sh` (or an explicit interpreter path available in the test env).
    #[test]
    fn installed_pre_commit_shebang_is_portable_sh() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);
        install_in(&repo);

        let script = std::fs::read_to_string(repo.join(".git/hooks/pre-commit")).unwrap();
        let first = script.lines().next().unwrap_or("");
        assert!(
            first == "#!/bin/sh" || first == "#!/bin/bash",
            "pre-commit shebang must be portable (/bin/sh or /bin/bash), not env bash; got: {first:?}\n\
             (nix sandbox often lacks /usr/bin/env → bash, causing No such file or directory)"
        );
        assert!(
            !first.contains("/usr/bin/env"),
            "pre-commit must not use #!/usr/bin/env … (missing in nix sandbox); got: {first}"
        );
    }

    #[test]
    fn pre_commit_blocks_staged_residual_csv_with_no_bypass_flag() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(repo.join("residual/stressors.csv"), "id\nS-01\n").unwrap();

        install_in(&repo);

        Command::new("git")
            .args(["add", "residual/stressors.csv"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let out = run_hook(&repo, "pre-commit");
        assert!(
            !out.status.success(),
            "pre-commit must refuse a staged residual/*.csv file"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("residual/stressors.csv"),
            "refusal must name the leaked path, got: {stderr}"
        );
        let script = std::fs::read_to_string(repo.join(".git/hooks/pre-commit")).unwrap();
        assert!(
            !script.contains("--force"),
            "the leak check must not offer a --force bypass"
        );
    }

    #[test]
    fn pre_commit_allows_residual_config_toml_alone() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(repo.join("residual/config.toml"), "format_version = \"v4\"\n").unwrap();

        install_in(&repo);

        Command::new("git")
            .args(["add", "residual/config.toml"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let out = run_hook(&repo, "pre-commit");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("refusing commit"),
            "residual/config.toml alone must not trip the leak block, stderr: {stderr}"
        );
    }

    #[test]
    fn install_writes_post_commit_hook() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);

        install_in(&repo);

        let content = std::fs::read_to_string(repo.join(".git/hooks/post-commit")).unwrap();
        assert!(content.contains("residual branch sync-commit"));
    }

    #[test]
    fn install_preserves_foreign_post_commit_hook_content() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);

        let hooks_dir = repo.join(".git/hooks");
        let foreign = "#!/bin/sh\n# Entire CLI hooks\nif command -v entire >/dev/null 2>&1; then entire hooks git post-commit 2>/dev/null || true; else :; fi\n";
        std::fs::write(hooks_dir.join("post-commit"), foreign).unwrap();

        install_in(&repo);

        let content = std::fs::read_to_string(hooks_dir.join("post-commit")).unwrap();
        assert!(
            content.contains("entire hooks git post-commit"),
            "install must preserve a foreign tool's existing post-commit hook, got:\n{content}"
        );
        assert!(
            content.contains("residual branch sync-commit"),
            "install must still append residual's own sync-commit call, got:\n{content}"
        );
    }

    #[test]
    fn reinstall_replaces_residual_block_in_place_without_duplicating() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        init_git_repo(&repo);

        let hooks_dir = repo.join(".git/hooks");
        let foreign = "#!/bin/sh\n# Entire CLI hooks\nentire hooks git post-commit 2>/dev/null || true\n";
        std::fs::write(hooks_dir.join("post-commit"), foreign).unwrap();

        install_in(&repo);
        install_in(&repo);

        let content = std::fs::read_to_string(hooks_dir.join("post-commit")).unwrap();
        assert_eq!(
            content.matches("residual branch sync-commit").count(),
            1,
            "reinstall must replace residual's block in place, not duplicate it, got:\n{content}"
        );
        assert_eq!(
            content.matches("entire hooks git post-commit").count(),
            1,
            "reinstall must not disturb the foreign tool's content, got:\n{content}"
        );
    }
}
