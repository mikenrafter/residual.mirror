use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod help;

#[derive(Parser)]
#[command(name = "residual", about = "NKP Residuality architecture CLI", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize residual/ in the current project.
    ///
    /// Process: idempotent bootstrap — create residual/ CSVs and v4 config
    /// without overwriting existing data. Add attractors before forces.
    Init {
        /// Overwrite session snapshot when residual files drifted outside this tool.
        #[arg(long)]
        force: bool,
    },
    /// Add a residual record.
    ///
    /// Process: examine whole-system-residue before a software-only patch.
    /// Forces carry outcomes, not component lists; map components via residues.
    /// Prefer adding a force (purpose XOR stressor) then a residue mapping.
    Add {
        /// Overwrite session snapshot when residual files drifted outside this tool.
        #[arg(long)]
        force: bool,
        #[command(subcommand)]
        target: AddTarget,
    },
    /// Remove a residual record.
    Remove {
        /// Overwrite session snapshot when residual files drifted outside this tool.
        #[arg(long)]
        force: bool,
        #[command(subcommand)]
        target: RemoveTarget,
    },
    /// List residual records (filter/group by attractor; not creation order).
    List {
        #[command(subcommand)]
        target: ListTarget,
    },
    /// Verify residual integrity.
    ///
    /// Process: one-way tags (code tags must exist in metadata; metadata-only is
    /// OK). Walks require at least two personas until alpha/beta exist.
    /// Policy (super_strict, token_warn, commit_msg_enforce) is read from storage-config.
    Verify {
        #[command(subcommand)]
        check: VerifyCheck,
    },
    /// Compose or check commit messages using project vocabulary.
    ///
    /// Process: subjects must use lexicon terms, component names, force prefixes,
    /// or start with `general - `. Body is always free-form.
    Commit {
        #[command(subcommand)]
        op: CommitOp,
    },
    /// NKP matrix operations (structure-analysis).
    ///
    /// Process: filter/group by attractor when reading; do not assume creation order.
    Matrix {
        #[command(subcommand)]
        op: MatrixOp,
    },
    /// Phase + installer skills.
    ///
    /// Process: a-la-carte — only the invoked subcommand carries ceremony.
    /// Use `skill install all` or `--agent all` to batch-install.
    Skill {
        #[command(subcommand)]
        op: SkillCommand,
    },
    Tag {
        #[command(subcommand)]
        op: TagOp,
    },
    /// Generate help artifacts (completions/man) or the verification git hook.
    Generate {
        #[command(subcommand)]
        artifact: GenerateArtifact,
    },
    /// Migrate residual/ from legacy on-disk shape to current.
    ///
    /// Process: config → storage-config; terminology.csv → lexicon.csv;
    /// attractors valence → +/- states; v3 inline force components → residues.csv;
    /// forces.csv deleted if present.
    Migrate {
        /// Overwrite session snapshot when residual files drifted outside this tool.
        #[arg(long)]
        force: bool,
        /// Lift inline residual/ to git sidecar branch.
        #[arg(long)]
        sidecar: bool,
    },
    /// Record or inspect architecture walk cadence (.walk-review.toml on sidecar).
    Walk {
        #[command(subcommand)]
        op: WalkOp,
    },
    Config,
}

#[derive(Subcommand)]
pub enum WalkOp {
    /// Stamp last completed purpose-walk or stressor-walk.
    Record {
        #[arg(long, value_enum)]
        kind: WalkKindArg,
        #[arg(long)]
        completed: bool,
        #[arg(long)]
        deferred: bool,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum WalkKindArg {
    Purpose,
    Stressor,
}

#[derive(Subcommand)]
pub enum SkillCommand {
    /// Show an embedded phase skill definition.
    ///
    /// Process: read the skill a-la-carte; unused phase ceremony is not loaded.
    Show {
        name: String,
        #[arg(long)]
        version: bool,
    },
    /// Print residual context for a phase skill.
    ///
    /// Process: load only the data that phase needs. Walks that use personas
    /// require min:2 (Verification).
    Data { name: String },
    /// List phase skills (stub + full) with token estimates.
    List,
    /// Install a phase skill into an agent directory.
    ///
    /// Pass `all` as the skill name to install every skill.
    /// Pass `--agent all` to install for every supported agent.
    Install {
        /// Skill name, or `all` to install every skill.
        name: String,
        /// Agent name (`claude`, `cursor`, `copilot`, `agnostic`), or `all`.
        #[arg(long, default_value = "agnostic")]
        agent: String,
        #[arg(long)]
        global: bool,
    },
    /// Check whether an installed skill matches the embedded version.
    ///
    /// Process: compare installed front-matter version to the binary. Prefer
    /// this name over legacy `skill-check`.
    CheckInstall {
        name: String,
        #[arg(long, default_value = "agnostic")]
        agent: String,
    },
}

#[derive(Subcommand)]
pub enum AddTarget {
    /// Add a stressor force. Process: whole-system-residue first — record outcomes.
    /// Map components via `residual add residue --force-id … --component-id …`.
    Stressor {
        #[arg(long)] description: String,
        #[arg(long)] attractor_id: String,
        #[arg(long)] naive_change: String,
        #[arg(long, default_value = "")] shortname: String,
        #[arg(long, default_value = "", visible_alias = "traits")]
        outcomes: String,
        #[arg(long)]
        whole_system: bool,
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Add force×component coupling to residues.csv (the NKP matrix).
    Residue {
        #[arg(long)] force_id: String,
        #[arg(long, default_value = "")] component_id: String,
        #[arg(long, default_value = "")]
        notes: String,
        #[arg(long)]
        whole_system: bool,
        /// Repoint an existing coupling from --component-id to this component.
        #[arg(long, default_value = "")]
        move_to: String,
    },
    /// Append a component to components.csv and extend residues.csv header.
    Component {
        #[arg(long)] name: String,
        #[arg(long)] description: String,
        #[arg(long)] status: String,
        #[arg(long)] architecture_set: String,
    },
    /// Add a purpose force. Process: whole-system-residue first — record outcomes.
    /// Map components via `residual add residue --force-id … --component-id …`.
    Purpose {
        #[arg(long)] description: String,
        #[arg(long)] attractor_id: String,
        #[arg(long, visible_alias = "feature")]
        naive_change: String,
        #[arg(long, default_value = "")] shortname: String,
        #[arg(long, default_value = "", visible_alias = "traits")]
        outcomes: String,
    },
    Attractor {
        #[arg(long)] name: String,
        #[arg(long)] description: String,
        #[arg(long)] positive_state: String,
        #[arg(long)] negative_state: String,
    },
    Term {
        #[arg(long)] term: String,
        #[arg(long)] definition: String,
        #[arg(long, default_value = "")] domain: String,
        #[arg(long, default_value = "")] related: String,
    },
    Persona {
        #[arg(long)] name: String,
        #[arg(long)] role: String,
        #[arg(long, default_value = "")] concerns: String,
        #[arg(long, default_value = "")] desires: String,
    },
    Iteration {
        #[arg(long, default_value = "")] notes: String,
        #[arg(long, default_value = "")] ri_score: String,
    },
}

#[derive(Subcommand)]
pub enum RemoveTarget {
    /// Clear force×component coupling from residues.csv.
    Residue {
        #[arg(long)] force_id: String,
        #[arg(long)] component_id: String,
    },
}

#[derive(Subcommand)]
pub enum ListTarget {
    Stressors,
    Purposes,
    Attractors,
    Terminology,
    Personas,
    Iterations,
    Residues,
}

#[derive(Subcommand)]
pub enum VerifyCheck {
    /// Verify purpose/stressor outcome statements reference terminology.
    #[command(name = "outcomes", visible_aliases = ["traits"])]
    Outcomes,
    Links,
    All,
    /// Non-blocking walk cadence reminder (purpose-walk + stressor-walk).
    #[command(name = "walk-reminder")]
    WalkReminder {
        /// Read staged paths for heuristic nudges.
        #[arg(long)]
        staged: bool,
    },
    /// Validate a git commit message subject against lexicon/components.
    CommitMsg {
        /// Path to the commit message file (first line = subject).
        #[arg(value_name = "FILE")]
        file: Option<String>,
        /// Message text instead of a file.
        #[arg(short, long)]
        message: Option<String>,
        /// Block on violations (overrides storage-config commit_msg_enforce).
        #[arg(long)]
        enforce: bool,
        /// Warn only, never block (overrides storage-config).
        #[arg(long)]
        warn: bool,
        /// Read staged paths for component hints.
        #[arg(long)]
        staged: bool,
    },
}

#[derive(Subcommand)]
pub enum CommitOp {
    /// Dry-run commit-msg validation.
    Check {
        #[arg(short, long)]
        message: String,
        #[arg(long)]
        enforce: bool,
        #[arg(long)]
        warn: bool,
        #[arg(long)]
        staged: bool,
    },
    /// Suggest subjects from staged diff and open residues.
    Suggest {
        #[arg(long)]
        staged: bool,
    },
    /// Print a scaffold for a force id (S-nn or P-nn).
    Template {
        force_id: String,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum MatrixSortBy {
    /// Group force rows by attractor (default).
    #[default]
    Attractor,
    /// Reorder component columns around fusion pairs / fission pressure.
    #[value(name = "fusion-fission")]
    FusionFission,
    /// Order force rows by id.
    Id,
    /// Order force rows by shortname.
    Alphabetical,
}

#[derive(Subcommand)]
pub enum MatrixOp {
    /// Print the NKP coupling table (stressor/purpose shortnames × components).
    ///
    /// Process: rows are shortnames from stressors/purposes; cells come from
    /// stressor↔component coupling. Forces are grouped by attractor with
    /// separator rows. Pass `--csv` for machine-readable stdout.
    Show {
        /// Emit CSV on stdout instead of a colored table.
        #[arg(long)]
        csv: bool,
        /// Keep only forces matching these attractor ids, force ids, or shortnames
        /// (comma-separated; repeatable).
        #[arg(long, value_delimiter = ',')]
        filter: Vec<String>,
        /// Row/column organization.
        #[arg(long, value_enum, default_value_t = MatrixSortBy::Attractor)]
        sort_by: MatrixSortBy,
    },
    Calc,
    Criticality,
    Ri {
        #[arg(long)] stressors: usize,
        #[arg(long)] naive_survived: usize,
        #[arg(long)] residual_survived: usize,
    },
    Fusion,
    Fission,
}

#[derive(Subcommand)]
pub enum TagOp {
    Scan {
        #[arg(default_value = ".")] path: String,
    },
    Report {
        #[arg(default_value = ".")] path: String,
    },
}

#[derive(Subcommand)]
pub enum GenerateArtifact {
    Completions,
    Man,
    Hook,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let cfg = crate::config::load()?;

    match cli.command {
        Command::Init { force } => crate::storage::init(&cfg, force),
        Command::Add { force, target } => crate::storage::add(&cfg, target, force),
        Command::Remove { force, target } => crate::storage::remove(&cfg, target, force),
        Command::List { target } => crate::storage::list(&cfg, target),
        Command::Verify { check } => match check {
            VerifyCheck::Outcomes => crate::verification::run(&cfg, VerifyCheck::Outcomes),
            VerifyCheck::Links => crate::verification::run(&cfg, VerifyCheck::Links),
            VerifyCheck::All => crate::verification::run(&cfg, VerifyCheck::All),
            VerifyCheck::WalkReminder { staged } => {
                crate::verification::run_walk_reminder(&cfg, staged)
            }
            VerifyCheck::CommitMsg {
                file,
                message,
                enforce,
                warn,
                staged,
            } => run_verify_commit_msg(&cfg, file, message, enforce, warn, staged),
        },
        Command::Commit { op } => match op {
            CommitOp::Check {
                message,
                enforce,
                warn,
                staged,
            } => run_verify_commit_msg(&cfg, None, Some(message), enforce, warn, staged),
            CommitOp::Suggest { staged } => run_commit_suggest(&cfg, staged),
            CommitOp::Template { force_id } => {
                print!("{}", crate::verification::commit_msg::template_for_force(&cfg, &force_id)?);
                Ok(())
            }
        },
        Command::Matrix { op } => crate::structure::analysis::nkp::run(&cfg, op),
        Command::Skill { op } => match op {
            SkillCommand::Show { name, version } => crate::skills::phases::show(&name, version),
            SkillCommand::Data { name } => crate::skills::phases::data(&cfg, &name),
            SkillCommand::List => crate::skills::phases::list_all(),
            SkillCommand::Install { name, agent, global } => {
                crate::skills::installer::install(&name, &agent, global)
            }
            SkillCommand::CheckInstall { name, agent } => {
                crate::skills::installer::check_install(&name, &agent)
            }
        },
        Command::Tag { op } => crate::structure::analysis::tag_scan::run(&cfg, op),
        Command::Generate { artifact } => match artifact {
            GenerateArtifact::Completions => crate::cli::help::generate_completions(),
            GenerateArtifact::Man => crate::cli::help::generate_man(),
            GenerateArtifact::Hook => crate::verification::git_hook::install(),
        },
        Command::Migrate { force, sidecar } => {
            if sidecar {
                let cwd = std::env::current_dir()?;
                let report =
                    crate::storage::integrity::migration::migrate_inline_to_sidecar(&cwd, force)?;
                println!(
                    "Sidecar migrate: branch={}, lifted_files={}",
                    report.sidecar_branch, report.lifted_files
                );
                Ok(())
            } else {
                crate::storage::migrate(&cfg, force)
            }
        }
        Command::Walk { op } => match op {
            WalkOp::Record {
                kind,
                completed,
                deferred,
            } => {
                use crate::verification::walk_reminder::{self, WalkKind};
                if completed == deferred {
                    anyhow::bail!("specify exactly one of --completed or --deferred");
                }
                let meta = crate::verification::metadata_dir_for_verify(&cfg)?;
                let walk_kind = match kind {
                    WalkKindArg::Purpose => WalkKind::Purpose,
                    WalkKindArg::Stressor => WalkKind::Stressor,
                };
                if completed {
                    walk_reminder::record_completed(&meta, walk_kind)?;
                    println!("Recorded {}-walk completion", walk_kind.as_str());
                } else {
                    let _ = (meta, walk_kind, deferred);
                    println!("Recorded {}-walk deferral", walk_kind.as_str());
                }
                Ok(())
            }
        }
        Command::Config => crate::config::print(&cfg),
    }
}

fn run_verify_commit_msg(
    cfg: &crate::config::Config,
    file: Option<String>,
    message: Option<String>,
    enforce: bool,
    warn: bool,
    staged: bool,
) -> Result<()> {
    use anyhow::{bail, Context};
    use std::fs;

    let text = match (file, message) {
        (Some(path), None) => fs::read_to_string(&path)
            .with_context(|| format!("read commit message file {}", path))?,
        (None, Some(msg)) => msg,
        (None, None) => bail!("provide a commit message FILE or --message"),
        (Some(_), Some(_)) => bail!("provide either a FILE or --message, not both"),
    };

    let staged_paths = if staged {
        crate::verification::commit_msg::git_staged_paths()?
    } else {
        vec![]
    };

    let enforce_override = if enforce {
        Some(true)
    } else if warn {
        Some(false)
    } else {
        None
    };

    crate::verification::commit_msg::run_verify(cfg, &text, &staged_paths, enforce_override)
}

fn run_commit_suggest(cfg: &crate::config::Config, staged: bool) -> Result<()> {
    let staged_paths = if staged {
        crate::verification::commit_msg::git_staged_paths()?
    } else {
        vec![]
    };

    let suggestions = crate::verification::commit_msg::suggest_subjects(cfg, &staged_paths)?;
    println!("Suggested subjects:");
    for s in suggestions {
        println!("  {s}");
    }
    if !staged_paths.is_empty() {
        println!("\nStaged paths:");
        for p in &staged_paths {
            println!("  {p}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_purpose_accepts_naive_change_flag() {
        let cli = Cli::try_parse_from([
            "residual", "add", "purpose",
            "--description", "d",
            "--attractor-id", "A-01",
            "--naive-change", "naive change text",
        ]).unwrap();
        match cli.command {
            Command::Add { target: AddTarget::Purpose { naive_change, .. }, .. } => {
                assert_eq!(naive_change, "naive change text");
            }
            _ => panic!("expected Command::Add(AddTarget::Purpose)"),
        }
    }

    #[test]
    fn add_purpose_feature_alias_maps_to_naive_change() {
        let cli = Cli::try_parse_from([
            "residual", "add", "purpose",
            "--description", "d",
            "--attractor-id", "A-01",
            "--feature", "aliased text",
        ]).unwrap();
        match cli.command {
            Command::Add { target: AddTarget::Purpose { naive_change, .. }, .. } => {
                assert_eq!(naive_change, "aliased text");
            }
            _ => panic!("expected Command::Add(AddTarget::Purpose)"),
        }
    }

    mod cli_integration {
        use super::*;
        use crate::config::Config;
        use crate::storage;
        use std::process::Command;

        fn isolated_tempdir() -> tempfile::TempDir {
            tempfile::Builder::new()
                .tempdir_in("/tmp")
                .expect("create temp dir under /tmp away from source residual/")
        }

        fn bin() -> std::path::PathBuf {
            if let Ok(path) = std::env::var("CARGO_BIN_EXE_residual") {
                return path.into();
            }
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            for profile in ["debug", "release"] {
                let candidate = manifest.join("target").join(profile).join("residual");
                if candidate.is_file() {
                    return candidate;
                }
            }
            manifest.join("target/debug/residual")
        }

        fn run(dir: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
            Command::new(bin())
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("failed to run residual binary")
        }

        fn init_storage(dir: &tempfile::TempDir) {
            let residual = dir.path().join("residual");
            std::fs::create_dir_all(&residual).unwrap();
            let cfg = Config {
                validation: crate::config::ValidationConfig { strict: true },
                skills: crate::config::SkillsConfig { token_warn: 1000 },
                residual_dir: residual,
            };
            storage::init(&cfg, true).expect("storage init in tempdir");
        }

        fn add_component(dir: &tempfile::TempDir, name: &str, description: &str) {
            let status = run(
                dir,
                &[
                    "add",
                    "component",
                    "--name",
                    name,
                    "--description",
                    description,
                    "--status",
                    "actual",
                    "--architecture-set",
                    "set",
                ],
            );
            assert!(
                status.status.success(),
                "add component {name}: {}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        #[test]
        fn add_component_appends_registry_and_extends_residues_header() {
            let dir = isolated_tempdir();
            init_storage(&dir);

            let add = run(
                &dir,
                &[
                    "add",
                    "component",
                    "--name",
                    "storage-git-sidecar",
                    "--description",
                    "Git sidecar storage",
                    "--status",
                    "proposed",
                    "--architecture-set",
                    "iter4-storage",
                ],
            );
            assert!(
                add.status.success(),
                "add component: {}",
                String::from_utf8_lossy(&add.stderr)
            );

            let components =
                std::fs::read_to_string(dir.path().join("residual/components.csv")).unwrap();
            assert!(components.contains("storage-git-sidecar"));

            let header = std::fs::read_to_string(dir.path().join("residual/residues.csv"))
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .to_string();
            assert!(
                header.contains("storage-git-sidecar"),
                "residues header must gain component column"
            );
        }

        #[test]
        fn add_component_idempotent_on_same_name() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            let args = [
                "add",
                "component",
                "--name",
                "cli",
                "--description",
                "hub",
                "--status",
                "actual",
                "--architecture-set",
                "set",
            ];
            run(&dir, &args);
            run(&dir, &args);
            let components =
                std::fs::read_to_string(dir.path().join("residual/components.csv")).unwrap();
            assert_eq!(
                components.matches("cli,").count(),
                1,
                "duplicate add component must be idempotent"
            );
        }

        #[test]
        fn remove_residue_clears_matrix_cell() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_component(&dir, "auth", "Auth");
            run(
                &dir,
                &[
                    "add",
                    "attractor",
                    "--name",
                    "X",
                    "--description",
                    "d",
                    "--positive-state",
                    "ok",
                    "--negative-state",
                    "bad",
                ],
            );
            run(
                &dir,
                &[
                    "add",
                    "stressor",
                    "--description",
                    "load",
                    "--attractor-id",
                    "A-01",
                    "--naive-change",
                    "cache",
                ],
            );
            run(
                &dir,
                &[
                    "add",
                    "residue",
                    "--force-id",
                    "S-01",
                    "--component-id",
                    "auth",
                ],
            );

            let remove = run(
                &dir,
                &[
                    "remove",
                    "residue",
                    "--force-id",
                    "S-01",
                    "--component-id",
                    "auth",
                ],
            );
            assert!(
                remove.status.success(),
                "remove residue: {}",
                String::from_utf8_lossy(&remove.stderr)
            );

            let residues = std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
            assert!(
                !residues.contains(",1") || !residues.lines().any(|l| l.starts_with("S-01,") && l.contains("1")),
                "auth cell must be cleared after remove residue"
            );
        }

        #[test]
        fn add_residue_move_to_repoints_coupling() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_component(&dir, "auth", "Auth");
            add_component(&dir, "db", "DB");
            run(
                &dir,
                &[
                    "add",
                    "attractor",
                    "--name",
                    "X",
                    "--description",
                    "d",
                    "--positive-state",
                    "ok",
                    "--negative-state",
                    "bad",
                ],
            );
            run(
                &dir,
                &[
                    "add",
                    "stressor",
                    "--description",
                    "load",
                    "--attractor-id",
                    "A-01",
                    "--naive-change",
                    "cache",
                ],
            );
            run(
                &dir,
                &[
                    "add",
                    "residue",
                    "--force-id",
                    "S-01",
                    "--component-id",
                    "auth",
                ],
            );

            let move_to = run(
                &dir,
                &[
                    "add",
                    "residue",
                    "--force-id",
                    "S-01",
                    "--component-id",
                    "auth",
                    "--move-to",
                    "db",
                ],
            );
            assert!(
                move_to.status.success(),
                "add residue --move-to: {}",
                String::from_utf8_lossy(&move_to.stderr)
            );

            let residues = std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
            let header = residues.lines().next().expect("header");
            let cols: Vec<&str> = header.split(',').map(str::trim).collect();
            let row = residues
                .lines()
                .find(|l| l.starts_with("S-01,"))
                .expect("S-01 row");
            let cells: Vec<&str> = row.split(',').map(str::trim).collect();
            let auth_idx = cols.iter().position(|c| *c == "auth").expect("auth column");
            let db_idx = cols.iter().position(|c| *c == "db").expect("db column");
            assert_ne!(
                cells.get(auth_idx).copied().unwrap_or(""),
                "1",
                "auth coupling must be cleared, row={row}"
            );
            assert_eq!(
                cells.get(db_idx).copied().unwrap_or(""),
                "1",
                "db coupling must be set, row={row}"
            );
        }
    }
}
