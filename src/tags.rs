use anyhow::Result;
use crate::config::Config;
use crate::cli::TagOp;
use std::collections::HashSet;

pub fn run(cfg: &Config, op: TagOp) -> Result<()> {
    match op {
        TagOp::Scan { path } => {
            let tags = scan_dir(&path)?;
            let report = scan_report(cfg, &tags)?;

            if report.dangling.is_empty() && report.untagged_stressors.is_empty() && report.untagged_purposes.is_empty() {
                println!("All tags valid and all forces tagged.");
            } else {
                for d in &report.dangling {
                    println!(
                        "DANGLING: {}:{} {} '{}' does not match any known {}",
                        d.file, d.line, d.kind.marker(), d.id, d.kind.target_description()
                    );
                }
                for shortname in &report.untagged_stressors {
                    println!("UNTAGGED stressor: {} (in storage but not referenced in code)", shortname);
                }
                for shortname in &report.untagged_purposes {
                    println!("UNTAGGED purpose: {} (in storage but not referenced in code)", shortname);
                }
            }
        }
        TagOp::Report { path } => {
            let tags = scan_dir(&path)?;
            if tags.is_empty() {
                println!("No tags found.");
            } else {
                for tag in &tags {
                    println!("{}:{} → {} {}", tag.file, tag.line, tag.kind.marker(), tag.ids.join(", "));
                }
            }
        }
    }
    Ok(())
}

pub struct Tag {
    pub file: String,
    pub line: usize,
    pub kind: TagKind,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// @component: <name> — code coupled to a component via the NKP matrix.
    Component,
    /// @stressor: <shortname> — code that exists to answer a specific stressor.
    Stressor,
    /// @purpose: <shortname> — code that exists to hold up a specific purpose.
    Purpose,
}

impl TagKind {
    pub fn marker(&self) -> &'static str {
        match self {
            TagKind::Component => "@component",
            TagKind::Stressor => "@stressor",
            TagKind::Purpose => "@purpose",
        }
    }

    fn target_description(&self) -> &'static str {
        match self {
            TagKind::Component => "component",
            TagKind::Stressor => "stressor",
            TagKind::Purpose => "purpose",
        }
    }
}

pub struct DanglingTag {
    pub file: String,
    pub line: usize,
    pub kind: TagKind,
    pub id: String,
}

pub struct ScanReport {
    pub dangling: Vec<DanglingTag>,
    pub untagged_stressors: Vec<String>,
    pub untagged_purposes: Vec<String>,
}

/// Cross-reference scanned tags against the ledger. `@stressor:`/`@purpose:` tags
/// must name a known force shortname (forces are shortname-only — A-19);
/// `@component:` tags must name a known component. Attractors are deliberately
/// not taggable — they describe system states, not code-adjacent detail.
/// Anything else is a dangling tag-shaped comment.
pub fn scan_report(cfg: &Config, tags: &[Tag]) -> Result<ScanReport> {
    let stressors = crate::storage::stressors::load(&cfg.residual_dir).unwrap_or_default();
    let stressor_shortnames: HashSet<String> = stressors
        .iter()
        .map(|s| s.shortname.clone())
        .filter(|s| !s.is_empty())
        .collect();

    let purposes = crate::storage::purposes::load(&cfg.residual_dir).unwrap_or_default();
    let purpose_shortnames: HashSet<String> = purposes
        .iter()
        .map(|p| p.shortname.clone())
        .filter(|s| !s.is_empty())
        .collect();

    let components = crate::structure::definition::components::load(&cfg.residual_dir).unwrap_or_default();
    let component_names: HashSet<String> = components.iter().map(|c| c.name.clone()).collect();

    let mut dangling = Vec::new();
    let mut tagged_stressor_shortnames: HashSet<String> = HashSet::new();
    let mut tagged_purpose_shortnames: HashSet<String> = HashSet::new();

    for tag in tags {
        let valid_targets = match tag.kind {
            TagKind::Stressor => &stressor_shortnames,
            TagKind::Purpose => &purpose_shortnames,
            TagKind::Component => &component_names,
        };
        for id in &tag.ids {
            match tag.kind {
                TagKind::Stressor => { tagged_stressor_shortnames.insert(id.clone()); }
                TagKind::Purpose => { tagged_purpose_shortnames.insert(id.clone()); }
                TagKind::Component => {}
            }
            if !valid_targets.contains(id) {
                dangling.push(DanglingTag {
                    file: tag.file.clone(),
                    line: tag.line,
                    kind: tag.kind,
                    id: id.clone(),
                });
            }
        }
    }

    let mut untagged_stressors: Vec<String> = stressor_shortnames
        .iter()
        .filter(|s| !tagged_stressor_shortnames.contains(*s))
        .cloned()
        .collect();
    untagged_stressors.sort();

    let mut untagged_purposes: Vec<String> = purpose_shortnames
        .iter()
        .filter(|s| !tagged_purpose_shortnames.contains(*s))
        .cloned()
        .collect();
    untagged_purposes.sort();

    Ok(ScanReport { dangling, untagged_stressors, untagged_purposes })
}

pub fn scan_dir(path: &str) -> Result<Vec<Tag>> {
    let root = std::path::Path::new(path);
    let mut tags = Vec::new();
    scan_path(root, &mut tags)?;
    Ok(tags)
}

/// Directory names that are never worth scanning for tags: build artifacts,
/// dependency trees, and nix-store symlinks (following `result` would walk the
/// entire store closure).
const SKIP_DIR_NAMES: &[&str] = &["target", "node_modules", "result", ".worktrees"];

fn scan_path(path: &std::path::Path, tags: &mut Vec<Tag>) -> Result<()> {
    // Never follow symlinks — avoids escaping the project tree (e.g. `result -> /nix/store/...`).
    let Ok(meta) = std::fs::symlink_metadata(path) else { return Ok(()) };
    if meta.file_type().is_symlink() {
        return Ok(());
    }

    if meta.is_dir() {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || SKIP_DIR_NAMES.contains(&name) {
                return Ok(());
            }
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            scan_path(&entry.path(), tags)?;
        }
    } else if meta.is_file() {
        scan_file(path, tags)?;
    }
    Ok(())
}

fn scan_file(path: &std::path::Path, tags: &mut Vec<Tag>) -> Result<()> {
    // Read as bytes first to detect binary files
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(()), // skip unreadable files
    };

    // Heuristic: skip binary files (contains null bytes)
    if bytes.contains(&0u8) {
        return Ok(());
    }

    let content = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => return Ok(()), // skip non-UTF8
    };

    // Comment syntax is per-language, and we don't want to hand-maintain that
    // list — ask tokei's language database instead (S-?: tag-scan-false-coverage
    // adjacent). Files with no recognized extension (CSV, unknown types) get an
    // empty token set, i.e. never scanned for tags, rather than guessing and
    // risking the same false-positive class fixed for residual/*.csv.
    let comment_tokens = comment_tokens_for(path);
    if comment_tokens.is_empty() {
        return Ok(());
    }

    let file_str = path.to_string_lossy().to_string();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        for (marker, kind) in [
            ("@component:", TagKind::Component),
            ("@stressor:", TagKind::Stressor),
            ("@purpose:", TagKind::Purpose),
        ] {
            if let Some(ids) = extract_tag(line, marker, &comment_tokens) {
                if !ids.is_empty() {
                    tags.push(Tag {
                        file: file_str.clone(),
                        line: line_num,
                        kind,
                        ids,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Comment-start tokens for a file, derived from tokei's language database
/// (line comments + multi-line comment openers) keyed by extension. Unknown
/// extensions yield an empty set rather than a guessed default.
fn comment_tokens_for(path: &std::path::Path) -> Vec<&'static str> {
    let lang = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(tokei::LanguageType::from_file_extension);

    match lang {
        Some(lang) => {
            let mut tokens: Vec<&'static str> = lang.line_comments().to_vec();
            tokens.extend(lang.multi_line_comments().iter().map(|(start, _)| *start));
            tokens
        }
        None => Vec::new(),
    }
}

/// Extract shortnames after a tag marker within a comment context.
/// Returns Some(ids) if the tag marker is found within an actual comment, None
/// otherwise. Legacy numeric-ID shapes (S-01, P-03, R-02) are filtered out — the
/// tagging system is shortname-only (A-19: names hold up as the ledger scales,
/// IDs don't).
fn extract_tag(line: &str, marker: &str, comment_tokens: &[&str]) -> Option<Vec<String>> {
    let pos = line.find(marker)?;
    if !preceded_by_comment_marker(&line[..pos], comment_tokens) {
        return None;
    }
    let after = &line[pos + marker.len()..];

    let ids: Vec<String> = after
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !is_id_shaped(s) && looks_like_shortname(s))
        .collect();

    Some(ids)
}

/// True for lowercase kebab-case shortnames: letters/digits, hyphen-separated,
/// no leading/trailing hyphen, no whitespace or punctuation. Real tags are
/// always terse shortnames — this rejects prose fragments that merely contain
/// the tag marker, e.g. a doc comment explaining the tag syntax itself.
fn looks_like_shortname(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// True when one of the file's comment-start tokens appears before the tag on
/// the line. Excludes ledger CSV rows and prose that merely mentions the tag
/// syntax — e.g. residual/*.csv stressor descriptions discussing tagging itself.
fn preceded_by_comment_marker(prefix: &str, comment_tokens: &[&str]) -> bool {
    comment_tokens.iter().any(|token| prefix.contains(token))
}

/// True for legacy `<Letter>-<digits>` id shapes: S-01, P-12, R-03, A-07.
fn is_id_shaped(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_uppercase() {
        return false;
    }
    if chars.next() != Some('-') {
        return false;
    }
    let rest = &s[2..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const RUST_COMMENTS: &[&str] = &["//", "/*"];

    fn cfg_for(dir: &std::path::Path) -> Config {
        Config {
            validation: crate::config::ValidationConfig { strict: true },
            skills: crate::config::SkillsConfig { token_warn: 1000 },
            residual_dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn scan_report_flags_stressor_tag_with_no_matching_shortname() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        let tags = vec![Tag {
            file: "src/example.rs".into(),
            line: 1,
            kind: TagKind::Stressor,
            ids: vec!["nonexistent-stressor".into()],
        }];
        let report = scan_report(&cfg, &tags).unwrap();
        assert_eq!(report.dangling.len(), 1);
        assert_eq!(report.dangling[0].id, "nonexistent-stressor");
        assert_eq!(report.dangling[0].kind, TagKind::Stressor);
    }

    #[test]
    fn scan_report_accepts_stressor_tag_matching_known_shortname() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        crate::storage::stressors::append(
            dir.path(),
            crate::storage::stressors::Stressor {
                id: "S-01".into(),
                shortname: "ceremony-lockout".into(),
                description: "d".into(),
                attractor_id: "A-01".into(),
                naive_change: "none".into(),
                outcomes: "".into(),
            },
        )
        .unwrap();
        let tags = vec![Tag {
            file: "src/example.rs".into(),
            line: 1,
            kind: TagKind::Stressor,
            ids: vec!["ceremony-lockout".into()],
        }];
        let report = scan_report(&cfg, &tags).unwrap();
        assert!(report.dangling.is_empty());
        assert!(report.untagged_stressors.is_empty());
    }

    #[test]
    fn scan_report_accepts_purpose_tag_matching_known_shortname() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        crate::storage::purposes::append(
            dir.path(),
            crate::storage::purposes::Purpose {
                id: "P-01".into(),
                shortname: "fluent-metadata-capture".into(),
                description: "d".into(),
                attractor_id: "A-01".into(),
                naive_change: "none".into(),
                outcomes: "".into(),
            },
        )
        .unwrap();
        let tags = vec![Tag {
            file: "src/example.rs".into(),
            line: 1,
            kind: TagKind::Purpose,
            ids: vec!["fluent-metadata-capture".into()],
        }];
        let report = scan_report(&cfg, &tags).unwrap();
        assert!(report.dangling.is_empty());
        assert!(report.untagged_purposes.is_empty());
    }

    #[test]
    fn scan_report_flags_purpose_tag_with_no_matching_shortname() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        let tags = vec![Tag {
            file: "src/example.rs".into(),
            line: 1,
            kind: TagKind::Purpose,
            ids: vec!["nonexistent-purpose".into()],
        }];
        let report = scan_report(&cfg, &tags).unwrap();
        assert_eq!(report.dangling.len(), 1);
        assert_eq!(report.dangling[0].kind, TagKind::Purpose);
    }

    #[test]
    fn scan_report_flags_component_tag_with_no_matching_component() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        let tags = vec![Tag {
            file: "src/example.rs".into(),
            line: 1,
            kind: TagKind::Component,
            ids: vec!["nonexistent-component".into()],
        }];
        let report = scan_report(&cfg, &tags).unwrap();
        assert_eq!(report.dangling.len(), 1);
        assert_eq!(report.dangling[0].kind, TagKind::Component);
    }

    #[test]
    fn scan_report_lists_shortnames_present_in_storage_but_untagged() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        crate::storage::stressors::append(
            dir.path(),
            crate::storage::stressors::Stressor {
                id: "S-01".into(),
                shortname: "phase-rigidity-assumption".into(),
                description: "d".into(),
                attractor_id: "A-01".into(),
                naive_change: "none".into(),
                outcomes: "".into(),
            },
        )
        .unwrap();
        let report = scan_report(&cfg, &[]).unwrap();
        assert_eq!(report.untagged_stressors, vec!["phase-rigidity-assumption".to_string()]);
        assert!(report.untagged_purposes.is_empty());
    }

    #[test]
    fn scan_path_does_not_follow_symlinks() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target_dir");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("evil.rs"), "// @stressor: ceremony-lockout").unwrap();
        let link = dir.path().join("link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(unix)]
        {
            let tags = scan_dir(&link.to_string_lossy()).unwrap();
            assert!(tags.is_empty(), "symlinked paths must not be scanned");
        }
    }

    #[test]
    fn scan_dir_skips_files_with_unrecognized_extension() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("residues.csv"), "// @stressor: ceremony-lockout\n").unwrap();
        let tags = scan_dir(&dir.path().to_string_lossy()).unwrap();
        assert!(tags.is_empty(), "unrecognized extensions (e.g. .csv) must not be scanned at all");
    }

    #[test]
    fn comment_tokens_for_rust_file_includes_line_and_block_comment() {
        let tokens = comment_tokens_for(std::path::Path::new("src/example.rs"));
        assert!(tokens.contains(&"//"));
        assert!(tokens.contains(&"/*"));
    }

    #[test]
    fn comment_tokens_for_python_file_uses_hash() {
        let tokens = comment_tokens_for(std::path::Path::new("script.py"));
        assert!(tokens.contains(&"#"));
    }

    #[test]
    fn comment_tokens_for_unrecognized_extension_is_empty() {
        let tokens = comment_tokens_for(std::path::Path::new("residual/residues.csv"));
        assert!(tokens.is_empty());
    }

    #[test]
    fn extract_tag_accepts_shortname() {
        let ids = extract_tag("// @stressor: ceremony-lockout", "@stressor:", RUST_COMMENTS).unwrap();
        assert_eq!(ids, vec!["ceremony-lockout".to_string()]);
    }

    #[test]
    fn extract_tag_rejects_legacy_id_shape() {
        let ids = extract_tag("// @stressor: S-24", "@stressor:", RUST_COMMENTS).unwrap();
        assert!(ids.is_empty(), "legacy IDs must not be accepted as tags, got {:?}", ids);
    }

    #[test]
    fn extract_tag_accepts_mixed_list_dropping_ids() {
        let ids = extract_tag(
            "// @stressor: ceremony-lockout, S-24, phase-rigidity-assumption",
            "@stressor:",
            RUST_COMMENTS,
        )
        .unwrap();
        assert_eq!(
            ids,
            vec!["ceremony-lockout".to_string(), "phase-rigidity-assumption".to_string()]
        );
    }

    #[test]
    fn extract_tag_requires_a_comment_token_for_this_file_type() {
        // No comment tokens supplied (e.g. unrecognized file type) → never matches.
        let ids = extract_tag("// @stressor: ceremony-lockout", "@stressor:", &[]);
        assert!(ids.is_none());
    }

    #[test]
    fn is_id_shaped_matches_letter_dash_digits() {
        assert!(is_id_shaped("S-01"));
        assert!(is_id_shaped("P-123"));
        assert!(is_id_shaped("A-7"));
        assert!(!is_id_shaped("ceremony-lockout"));
        assert!(!is_id_shaped("skills-phases"));
        assert!(!is_id_shaped(""));
    }

    #[test]
    fn extract_tag_rejects_prose_that_merely_mentions_the_marker() {
        // A doc comment explaining the tag syntax itself must not be picked up
        // as an applied tag (self-referential false positive in tags.rs/verify.rs).
        let ids = extract_tag(
            "/// @stressor: tags must name a known shortname, not free text.",
            "@stressor:",
            RUST_COMMENTS,
        )
        .unwrap();
        assert!(ids.is_empty(), "prose after the marker must not parse as ids, got {:?}", ids);
    }

    #[test]
    fn looks_like_shortname_rejects_prose_and_punctuation() {
        assert!(looks_like_shortname("ceremony-lockout"));
        assert!(looks_like_shortname("skills-phases"));
        assert!(!looks_like_shortname("tags must name a"));
        assert!(!looks_like_shortname("\"@stressor:\").unwrap();"));
        assert!(!looks_like_shortname(""));
        assert!(!looks_like_shortname("-leading-hyphen"));
        assert!(!looks_like_shortname("trailing-hyphen-"));
    }
}
