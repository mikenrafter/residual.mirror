//! Migration — legacy on-disk shapes to current.
//!
//! Converts:
//! - config.toml (`[validation]`/`[skills]` → storage-config)
//! - terminology.csv → lexicon.csv (related_terms → aliases)
//! - attractors.csv (valence/phase_state → positive_state/negative_state)
//! - v3 → v4: inline force `components` columns → matrix `residues.csv` (NKP source)

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::storage::config::StorageConfig;
use crate::storage::format::{self, write_attractors_v3};
use crate::structure::analysis::attractors::Attractor as V3Attractor;
use crate::structure::analysis::residues::Residue;
use crate::structure::definition::lexicon::Term as LexiconTerm;

#[derive(Debug, Clone)]
pub struct MigratedV3 {
    pub format_version: String,
    pub storage: StorageConfig,
    pub toml: String,
}

#[derive(Debug, Clone, Default)]
pub struct MigrateReport {
    pub config_migrated: bool,
    pub attractors: usize,
    pub lexicon_terms: usize,
    /// v3→v4: force×component couplings written to residues.csv.
    pub v4_residue_couplings: usize,
    pub v4_forces_decoupled: bool,
}

#[derive(Debug, Deserialize)]
struct NaiveDocument {
    #[serde(default)]
    validation: NaiveValidation,
    #[serde(default)]
    skills: NaiveSkills,
}

#[derive(Debug, Default, Deserialize)]
struct NaiveValidation {
    #[serde(default = "default_true")]
    strict: bool,
}

#[derive(Debug, Default, Deserialize)]
struct NaiveSkills {
    #[serde(default = "default_token_warn")]
    token_warn: usize,
}

fn default_true() -> bool {
    true
}
fn default_token_warn() -> usize {
    1000
}

/// Convert naive config.toml (`[validation]` / `[skills]`) into v3 TOML.
pub fn migrate_naive_to_v3(naive_toml: &str) -> Result<MigratedV3> {
    let naive: NaiveDocument =
        toml::from_str(naive_toml).with_context(|| "parse naive config.toml")?;
    let storage = StorageConfig {
        format_version: "v3".to_string(),
        change_detection: true,
        super_strict: naive.validation.strict,
        token_warn: naive.skills.token_warn,
        commit_msg_enforce: false,
    };
    let toml_out = crate::storage::config::render_v3(&storage);
    Ok(MigratedV3 {
        format_version: storage.format_version.clone(),
        storage,
        toml: format!(
            "# residual v3 configuration (migrated from naive)\n{}",
            toml_out.trim_start_matches("# residual v3 configuration\n")
        ),
    })
}

fn is_v3_config(raw: &str) -> bool {
    raw.contains("format_version") || raw.contains("[verification]") || raw.contains("[storage]")
}

fn load_naive_attractors(path: &Path) -> Result<Vec<(String, String, String, String, String)>> {
    // id, name, valence, description, phase_state
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path)?;
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        rows.push((
            rec.get(0).unwrap_or("").to_string(),
            rec.get(1).unwrap_or("").to_string(),
            rec.get(2).unwrap_or("").to_string(),
            rec.get(3).unwrap_or("").to_string(),
            rec.get(4).unwrap_or("").to_string(),
        ));
    }
    Ok(rows)
}

fn migrate_attractor_row(
    id: String,
    name: String,
    valence: &str,
    description: String,
    phase_state: String,
) -> V3Attractor {
    let (positive_state, negative_state) = match valence.to_lowercase().as_str() {
        "positive" => {
            let pos = if phase_state.is_empty() {
                description.clone()
            } else {
                phase_state
            };
            let neg = if description.is_empty() {
                format!("(migrated) pressure fails for {name}")
            } else {
                format!("(migrated) pressure fails when: {description}")
            };
            (pos, neg)
        }
        "negative" => {
            let neg = if phase_state.is_empty() {
                description.clone()
            } else {
                phase_state
            };
            let pos = if description.is_empty() {
                format!("(migrated) pressure holds for {name}")
            } else {
                format!("(migrated) pressure holds when opposite of: {description}")
            };
            (pos, neg)
        }
        _ => {
            let pos = if phase_state.is_empty() {
                format!("(migrated) positive state for {name}")
            } else {
                phase_state
            };
            let neg = if description.is_empty() {
                format!("(migrated) negative state for {name}")
            } else {
                description.clone()
            };
            (pos, neg)
        }
    };
    V3Attractor {
        id,
        name,
        description,
        positive_state,
        negative_state,
    }
}

/// Migrate a residual/ directory to current on-disk shape.
pub fn migrate_residual_dir(residual_dir: &Path, force: bool) -> Result<MigrateReport> {
    if !residual_dir.is_dir() {
        bail!("residual dir not found: {}", residual_dir.display());
    }

    let session = crate::storage::integrity::sessions::begin_mutation(residual_dir, force)?;
    let mut report = MigrateReport::default();

    // --- config ---
    let config_path = residual_dir.join("config.toml");
    if config_path.exists() {
        let raw = fs::read_to_string(&config_path)?;
        if !is_v3_config(&raw) {
            let migrated = migrate_naive_to_v3(&raw)?;
            fs::write(&config_path, &migrated.toml)?;
            report.config_migrated = true;
        }
    } else {
        let cfg = StorageConfig::default();
        fs::write(&config_path, crate::storage::config::render_v3(&cfg))?;
        report.config_migrated = true;
    }

    // --- terminology.csv → lexicon.csv ---
    let terminology_path = residual_dir.join("terminology.csv");
    if terminology_path.exists() {
        #[derive(serde::Deserialize)]
        struct OldTerm {
            term: String,
            definition: String,
            domain: String,
            related_terms: String,
        }
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&terminology_path)?;
        let mut lexicon: Vec<LexiconTerm> = format::read_lexicon(residual_dir)?;
        let existing_terms: std::collections::HashSet<String> =
            lexicon.iter().map(|t| t.term.clone()).collect();
        let mut added = 0usize;
        for rec in rdr.deserialize() {
            let t: OldTerm = rec?;
            if !existing_terms.contains(&t.term) {
                lexicon.push(LexiconTerm {
                    term: t.term,
                    definition: t.definition,
                    domain: t.domain,
                    aliases: t.related_terms,
                });
                added += 1;
            }
        }
        if added > 0 {
            format::write_lexicon(residual_dir, &lexicon)?;
        }
        fs::remove_file(&terminology_path)?;
        report.lexicon_terms = lexicon.len();
    }

    // --- forces.csv — delete if present (stranded migration artifact) ---
    let forces_path = residual_dir.join("forces.csv");
    if forces_path.exists() {
        fs::remove_file(&forces_path)?;
    }

    // --- v3 → v4: inline force components → residues.csv (before force CSV rewrite) ---
    let config_path = residual_dir.join("config.toml");
    let format_version = if config_path.exists() {
        crate::storage::config::parse_v3(&fs::read_to_string(&config_path)?)?
            .format_version
    } else {
        String::new()
    };
    if format_version != "v4" {
        let (n, decoupled) = migrate_v3_to_v4_residues(residual_dir)?;
        report.v4_residue_couplings = n;
        report.v4_forces_decoupled = decoupled;
    }

    // --- stressors.csv — normalize to current column names ---
    let stressors_path = residual_dir.join("stressors.csv");
    if stressors_path.exists() {
        let stressors = crate::storage::stressors::load(residual_dir)?;
        crate::storage::stressors::write_all_pub(residual_dir, &stressors)?;
    }

    // --- purposes.csv — normalize to current column names ---
    let purposes_path = residual_dir.join("purposes.csv");
    if purposes_path.exists() {
        let purposes = crate::storage::purposes::load(residual_dir)?;
        crate::storage::purposes::write_all_pub(residual_dir, &purposes)?;
    }

    // --- attractors valence → +/- states ---
    let attractors_path = residual_dir.join("attractors.csv");
    if attractors_path.exists() {
        let header = {
            let text = fs::read_to_string(&attractors_path)?;
            text.lines().next().unwrap_or("").to_string()
        };
        let v3_attractors = if header.contains("positive_state") {
            format::read_attractors_v3(residual_dir)?
        } else {
            let naive_rows = load_naive_attractors(&attractors_path)?;
            naive_rows
                .into_iter()
                .map(|(id, name, valence, description, phase_state)| {
                    migrate_attractor_row(id, name, &valence, description, phase_state)
                })
                .collect()
        };
        write_attractors_v3(residual_dir, &v3_attractors)?;
        report.attractors = v3_attractors.len();
    }

    session.commit()?;

    Ok(report)
}

#[derive(serde::Deserialize)]
struct LegacyForceRow {
    id: String,
    #[serde(default, alias = "components_affected", alias = "components_enabled")]
    components: String,
}

fn csv_header_has(path: &Path, column: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let header = fs::read_to_string(path)?
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    Ok(header
        .split(',')
        .any(|h| h.trim().eq_ignore_ascii_case(column)))
}

fn legacy_force_components(path: &Path) -> Result<Vec<(String, String)>> {
    if !csv_header_has(path, "components")? {
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path)?;
    let mut out = Vec::new();
    for rec in rdr.deserialize() {
        let row: LegacyForceRow = rec?;
        out.push((row.id, row.components));
    }
    Ok(out)
}

fn ingest_component_field(couplings: &mut BTreeSet<(String, String)>, force_id: &str, field: &str) {
    for comp in field.split(',') {
        let comp = comp.trim();
        if !comp.is_empty() {
            couplings.insert((force_id.to_string(), comp.to_string()));
        }
    }
}

/// Move inline force `components` columns into matrix-shaped `residues.csv` (v4).
fn migrate_v3_to_v4_residues(residual_dir: &Path) -> Result<(usize, bool)> {
    let mut couplings: BTreeSet<(String, String)> = BTreeSet::new();
    let mut decoupled = false;

    for (force_id, comps) in legacy_force_components(&residual_dir.join("stressors.csv"))? {
        ingest_component_field(&mut couplings, &force_id, &comps);
        decoupled = true;
    }
    for (force_id, comps) in legacy_force_components(&residual_dir.join("purposes.csv"))? {
        ingest_component_field(&mut couplings, &force_id, &comps);
        decoupled = true;
    }

    for r in format::read_residues(residual_dir)? {
        if r.is_coupled() {
            couplings.insert((r.force_id.clone(), r.component_id.clone()));
        }
    }

    let residues: Vec<Residue> = couplings
        .into_iter()
        .enumerate()
        .map(|(i, (force_id, component_id))| {
            if component_id == crate::structure::analysis::residues::WHOLE_SYSTEM_COMPONENT {
                Residue::whole_system(format!("R-{:02}", i + 1), force_id, "")
            } else {
                Residue::coupling(format!("R-{:02}", i + 1), force_id, component_id)
            }
        })
        .collect();
    let count = residues.len();
    format::write_residues(residual_dir, &residues)?;

    if decoupled {
        let stressors = crate::storage::stressors::load(residual_dir)?;
        crate::storage::stressors::write_all_pub(residual_dir, &stressors)?;
        let purposes = crate::storage::purposes::load(residual_dir)?;
        crate::storage::purposes::write_all_pub(residual_dir, &purposes)?;
    }

    let config_path = residual_dir.join("config.toml");
    let mut cfg = if config_path.exists() {
        crate::storage::config::parse_v3(&fs::read_to_string(&config_path)?)?
    } else {
        StorageConfig::default()
    };
    cfg.format_version = "v4".to_string();
    fs::write(&config_path, crate::storage::config::render_v3(&cfg))?;

    Ok((count, decoupled))
}

#[derive(Debug, Clone, Default)]
pub struct MigrateSidecarReport {
    pub sidecar_branch: String,
    pub config_path: PathBuf,
    pub lifted_files: usize,
}

/// Lift inline working-tree residual/ to sidecar branch (migrate --sidecar).
pub fn migrate_inline_to_sidecar(repo_root: &Path, force: bool) -> Result<MigrateSidecarReport> {
    let residual_dir = repo_root.join("residual");
    if !residual_dir.is_dir() {
        bail!("no residual/ directory at {}", residual_dir.display());
    }

    let branch = "residual/metadata".to_string();
    let config_path = residual_dir.join("config.toml");
    let prev = git_current_branch(repo_root)?;

    if git_branch_exists(repo_root, &branch)? {
        if force {
            let out = git_cmd(repo_root)
                .args(["branch", "-D", &branch])
                .output()
                .context("git branch -D sidecar")?;
            if !out.status.success() {
                bail!(
                    "git branch -D {branch} failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        } else {
            bail!(
                "sidecar branch {branch} already exists; use --force to recreate metadata-only branch"
            );
        }
    }

    let orphan_out = git_cmd(repo_root)
        .args(["checkout", "--orphan", &branch])
        .output()
        .context("git checkout --orphan sidecar")?;
    if !orphan_out.status.success() {
        bail!(
            "git checkout --orphan {branch} failed: {}",
            String::from_utf8_lossy(&orphan_out.stderr)
        );
    }

    let sidecar_toml = enable_sidecar_in_toml(&fs::read_to_string(&config_path).unwrap_or_default());
    fs::write(&config_path, &sidecar_toml)?;

    git_cmd(repo_root)
        .args(["rm", "-rf", "--cached", "."])
        .output()
        .ok();
    git_cmd(repo_root)
        .args(["add", "-f", "residual/"])
        .output()
        .context("git add residual on sidecar")?;
    let commit_out = git_cmd(repo_root)
        .args([
            "commit",
            "-m",
            "storage: S-01: migrate inline residual to sidecar branch",
        ])
        .output()
        .context("git commit sidecar migration")?;
    if !commit_out.status.success() {
        bail!(
            "git commit sidecar migration failed: {}",
            String::from_utf8_lossy(&commit_out.stderr)
        );
    }

    let list_out = git_cmd(repo_root)
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .context("git ls-tree sidecar branch")?;
    let tree_raw = String::from_utf8_lossy(&list_out.stdout);
    let tree_paths: Vec<&str> = tree_raw
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    if tree_paths.iter().any(|p| !p.starts_with("residual/")) {
        bail!(
            "sidecar branch must contain only residual/ paths; found: {}",
            tree_paths
                .iter()
                .filter(|p| !p.starts_with("residual/"))
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let lifted_files = tree_paths
        .iter()
        .filter(|p| !p.ends_with('/'))
        .count();

    let checkout = git_cmd(repo_root)
        .args(["checkout", "-f", &prev])
        .output()
        .context("git checkout previous branch")?;
    if !checkout.status.success() {
        bail!(
            "git checkout {prev} failed: {}",
            String::from_utf8_lossy(&checkout.stderr)
        );
    }

    Ok(MigrateSidecarReport {
        sidecar_branch: branch,
        config_path,
        lifted_files,
    })
}

fn git_cmd(repo_root: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(repo_root);
    cmd
}

fn git_branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let out = git_cmd(repo_root)
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .output()
        .context("git rev-parse branch")?;
    Ok(out.status.success())
}

fn git_current_branch(repo_root: &Path) -> Result<String> {
    let out = git_cmd(repo_root)
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .context("git symbolic-ref")?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    Ok("main".to_string())
}

fn enable_sidecar_in_toml(raw: &str) -> String {
    if raw.contains("git_sidecar_enabled") {
        return raw.to_string();
    }
    let mut cfg = if raw.trim().is_empty() {
        StorageConfig::default()
    } else if let Ok(c) = crate::storage::config::parse_v3(raw) {
        c
    } else {
        StorageConfig::default()
    };
    cfg.format_version = "v4".to_string();
    format!(
        "# residual v4 configuration\nformat_version = \"{}\"\n\n[storage]\nchange_detection = {}\ngit_sidecar_enabled = true\ngit_sidecar_branch = \"residual/metadata\"\ngit_sidecar_remote = \"origin\"\n\n[storage.git_sidecar]\nworking_tree_policy = \"warn\"\n\n[verification]\nsuper_strict = {}\ntoken_warn = {}\ncommit_msg_enforce = {}\n",
        cfg.format_version,
        cfg.change_detection,
        cfg.super_strict,
        cfg.token_warn,
        cfg.commit_msg_enforce
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migration_naive_to_v3() {
        let naive = "[validation]\nstrict = false\n\n[skills]\ntoken_warn = 500\n";
        let v3 = migrate_naive_to_v3(naive).unwrap();
        assert_eq!(v3.format_version, "v3");
        assert!(v3.storage.change_detection);
        assert!(!v3.storage.super_strict);
        assert_eq!(v3.storage.token_warn, 500);
        assert!(v3.toml.contains("format_version = \"v3\""));
        assert!(v3.toml.contains("[storage]"));
        assert!(v3.toml.contains("[verification]"));
        assert!(v3.toml.contains("super_strict = false"));
        assert!(v3.toml.contains("token_warn = 500"));
    }

    #[test]
    fn migrate_residual_dir_migrates_terminology_and_attractors() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        fs::create_dir_all(&residual).unwrap();
        fs::write(
            residual.join("config.toml"),
            "[validation]\nstrict = true\n\n[skills]\ntoken_warn = 1000\n",
        )
        .unwrap();
        fs::write(
            residual.join("terminology.csv"),
            "term,definition,domain,related_terms\nresidue,unit of change,core,\nstressor,narrative,core,\n",
        )
        .unwrap();
        fs::write(
            residual.join("attractors.csv"),
            "id,name,valence,description,phase_state\nA-01,Clarity,positive,NKP reflects reality,data is coherent\nA-02,Drift,negative,terms go stale,traits fail\n",
        )
        .unwrap();

        let report = migrate_residual_dir(&residual, true).unwrap();
        assert!(report.config_migrated);
        assert_eq!(report.attractors, 2);
        assert_eq!(report.lexicon_terms, 2);

        let cfg = fs::read_to_string(residual.join("config.toml")).unwrap();
        assert!(cfg.contains("format_version = \"v4\""));

        assert!(!residual.join("terminology.csv").exists(), "terminology.csv should be deleted");

        let lexicon = format::read_lexicon(&residual).unwrap();
        assert_eq!(lexicon.len(), 2);

        let attractors = format::read_attractors_v3(&residual).unwrap();
        assert_eq!(attractors.len(), 2);
        assert!(!attractors[0].positive_state.is_empty());
        let header = fs::read_to_string(residual.join("attractors.csv"))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string();
        assert!(header.contains("positive_state"));
        assert!(!header.contains("valence"));
    }

    #[test]
    fn migrate_deletes_forces_csv_if_present() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        fs::create_dir_all(&residual).unwrap();
        fs::write(residual.join("config.toml"), "# residual v3 configuration\nformat_version = \"v3\"\n\n[storage]\n\n[verification]\n").unwrap();
        fs::write(residual.join("forces.csv"), "id,kind,shortname\nS-01,stressor,foo\n").unwrap();
        let _report = migrate_residual_dir(&residual, true).unwrap();
        assert!(!residual.join("forces.csv").exists(), "forces.csv should be deleted by migrate");
    }

    #[test]
    fn migrate_v3_force_components_to_residues_matrix() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        fs::create_dir_all(&residual).unwrap();
        fs::write(
            residual.join("config.toml"),
            "# residual v3 configuration\nformat_version = \"v3\"\n\n[storage]\n\n[verification]\n",
        )
        .unwrap();
        fs::write(
            residual.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,components,attractor_id\n\
S-01,alpha,desc,change,outcome text,\"auth,db\",A-01\n",
        )
        .unwrap();
        fs::write(
            residual.join("purposes.csv"),
            "id,shortname,description,naive_change,outcomes,components,attractor_id\n",
        )
        .unwrap();
        fs::write(
            residual.join("residues.csv"),
            "force,auth,db\n",
        )
        .unwrap();

        let report = migrate_residual_dir(&residual, true).unwrap();
        assert!(report.v4_forces_decoupled);
        assert_eq!(report.v4_residue_couplings, 2);

        let cfg = fs::read_to_string(residual.join("config.toml")).unwrap();
        assert!(cfg.contains("format_version = \"v4\""));

        let stressors = fs::read_to_string(residual.join("stressors.csv")).unwrap();
        assert!(!stressors.contains(",components,"));
        assert!(!stressors.lines().nth(1).unwrap_or("").contains("auth"));

        let residues = fs::read_to_string(residual.join("residues.csv")).unwrap();
        assert!(residues.starts_with("force,"));
        assert!(residues.contains("S-01"));
        assert!(residues.contains("auth"));
        assert!(residues.contains("db"));
    }

    #[test]
    fn migrate_sidecar_lifts_inline_to_branch() {
        use std::process::Command;

        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("residual")).unwrap();
        std::fs::write(
            repo.join("residual/config.toml"),
            "# residual v4 configuration\nformat_version = \"v4\"\n\n[storage]\nchange_detection = true\n\n[verification]\n",
        )
        .unwrap();
        std::fs::write(
            repo.join("residual/stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-01,alpha,desc,change,outcome,A-01\n",
        )
        .unwrap();

        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&repo)
            .output()
            .expect("git init");
        Command::new("git")
            .args(["config", "user.email", "t@example.com"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "inline residual"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let report = migrate_inline_to_sidecar(&repo, true).unwrap();
        assert_eq!(report.sidecar_branch, "residual/metadata");
        assert!(report.lifted_files > 0, "must lift at least stressors.csv to sidecar");

        let branch_out = Command::new("git")
            .args(["branch", "--list", "residual/metadata"])
            .current_dir(&repo)
            .output()
            .unwrap();
        let branch_list = String::from_utf8_lossy(&branch_out.stdout);
        assert!(
            branch_list.contains("residual/metadata"),
            "migrate --sidecar must create sidecar branch"
        );

        let tree_out = Command::new("git")
            .args(["ls-tree", "-r", "--name-only", "residual/metadata"])
            .current_dir(&repo)
            .output()
            .unwrap();
        let tree = String::from_utf8_lossy(&tree_out.stdout);
        for line in tree.lines().filter(|l| !l.is_empty()) {
            assert!(
                line.starts_with("residual/"),
                "sidecar branch must be metadata-only; found {line}"
            );
        }
        assert!(
            !tree.contains("Cargo.toml"),
            "sidecar branch must not include application source tree"
        );
    }
}
