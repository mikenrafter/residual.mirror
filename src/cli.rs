use anyhow::{Context, Result};
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
    /// Update fields on an existing residual record in place.
    ///
    /// Process: keyed by the same id/name the record was created with. Every
    /// other field flag is optional — omitted flags leave the stored value
    /// unchanged. Stressor/purpose also accept repeatable --add-component /
    /// --remove-component to fold residues.csv couplings into the same call.
    Update {
        /// Overwrite session snapshot when residual files drifted outside this tool.
        #[arg(long)]
        force: bool,
        #[command(subcommand)]
        target: UpdateTarget,
    },
    /// Grant short-lived permission for sidecar-backed metadata writes.
    Write {
        #[command(subcommand)]
        op: WriteOp,
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
    /// Agent lifecycle hooks (pre-compaction, post-compaction, periodic turn).
    ///
    /// Process: an adapter boundary for agent tooling, distinct from explicit skill
    /// sessions. See skill-hooks (P-33 residual-ledger-continuity, P-34
    /// contextual-guru-cadence). Does not register with any agent's config.
    Hook {
        #[command(subcommand)]
        op: HookOp,
    },
    /// Manage per-code-branch metadata branches (git sidecar branch mode).
    ///
    /// `init` is the only subcommand that creates a metadata branch — every other
    /// mutating command requires it to already exist and points here if it doesn't.
    Branch {
        #[command(subcommand)]
        op: BranchOp,
    },
    /// Render the force landscape as a self-contained HTML page and print its path.
    ///
    /// Process: read-only and ephemeral — the page stages `residual add …`
    /// commands for the operator to copy, it never mutates the ledger.
    View {
        /// Include the defense ledger (meta forces + defense artifacts).
        #[arg(long)]
        defense: bool,
        /// Write the page here instead of a temp dir.
        #[arg(long, value_name = "DIR")]
        out: Option<std::path::PathBuf>,
    },
    /// Serve the force landscape on loopback and print the URL.
    Serve {
        /// Include the defense ledger (meta forces + defense artifacts).
        #[arg(long)]
        defense: bool,
        #[arg(long, default_value_t = crate::view::DEFAULT_PORT)]
        port: u16,
    },
    Config,
}

#[derive(Subcommand)]
pub enum BranchOp {
    /// Fork a metadata branch for the current (or given) code branch from trunk.
    Init { branch: Option<String> },
    /// Copy the resolved metadata branch's tree into the config-host residual/ dir for manual editing.
    Edit,
    /// Save hand-edited metadata back to the resolved metadata branch.
    Save {
        #[arg(long, default_value = "")]
        commit_msg: String,
        #[arg(long)]
        push: bool,
    },
    /// Merge one metadata branch into another (resolved from code branch names).
    Merge {
        from: String,
        to: String,
        #[arg(long)]
        commit_msg: Option<String>,
        #[arg(long)]
        push: bool,
    },
    /// Mirror the latest commit into the resolved metadata branch (post-commit hook).
    #[command(name = "sync-commit")]
    SyncCommit,
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
pub enum HookOp {
    /// Short prompt instructing the compaction process what to preserve.
    #[command(name = "pre-compaction")]
    PreCompaction,
    /// Short reminder to consult skills-guru before deciding next steps.
    #[command(name = "post-compaction")]
    PostCompaction,
    /// Classify a discussion snippet against skills-guru topics and, subject to
    /// per-topic cadence and dedup, print a suggestion (silent if none applies).
    #[command(name = "periodic-turn")]
    PeriodicTurn {
        #[arg(long)]
        turn: u32,
        /// Discussion snippet to classify; reads stdin when omitted.
        #[arg(long)]
        text: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum WriteOp {
    /// Authorize metadata writes for 30 minutes by default.
    Authorize {
        #[arg(long, default_value_t = crate::storage::write_authorization::DEFAULT_MINUTES)]
        minutes: i64,
    },
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
        #[arg(long)]
        description: String,
        #[arg(long)]
        attractor_id: String,
        #[arg(long)]
        naive_change: String,
        #[arg(long, default_value = "")]
        shortname: String,
        #[arg(long, default_value = "", visible_alias = "traits")]
        outcomes: String,
        #[arg(long)]
        whole_system: bool,
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Add force×component coupling to residues.csv (the NKP matrix).
    Residue {
        #[arg(long)]
        force_id: String,
        #[arg(long, default_value = "")]
        component_id: String,
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
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        architecture_set: String,
    },
    /// Add a purpose force. Process: whole-system-residue first — record outcomes.
    /// Map components via `residual add residue --force-id … --component-id …`.
    Purpose {
        #[arg(long)]
        description: String,
        #[arg(long)]
        attractor_id: String,
        #[arg(long, visible_alias = "feature")]
        naive_change: String,
        #[arg(long, default_value = "")]
        shortname: String,
        #[arg(long, default_value = "", visible_alias = "traits")]
        outcomes: String,
    },
    Attractor {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        positive_state: String,
        #[arg(long)]
        negative_state: String,
    },
    Term {
        #[arg(long)]
        term: String,
        #[arg(long)]
        definition: String,
        #[arg(long, default_value = "")]
        domain: String,
        #[arg(long, default_value = "")]
        related: String,
    },
    Persona {
        #[arg(long)]
        name: String,
        #[arg(long)]
        role: String,
        #[arg(long, default_value = "")]
        concerns: String,
        #[arg(long, default_value = "")]
        desires: String,
    },
    Iteration {
        #[arg(long, default_value = "")]
        notes: String,
        #[arg(long, default_value = "")]
        ri_score: String,
    },
    /// Add a meta-stressor (MS-*) to defense/meta-stressors.csv only.
    MetaStressor {
        #[arg(long)]
        description: String,
        #[arg(long, default_value = "")]
        shortname: String,
    },
    /// Add a meta-attractor (MA-*) to defense/meta-attractors.csv only.
    MetaAttractor {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        positive_state: String,
        #[arg(long)]
        negative_state: String,
    },
    /// Add a meta-purpose (MP-*) to defense/meta-purposes.csv only.
    MetaPurpose {
        #[arg(long)]
        description: String,
        #[arg(long)]
        attractor_id: String,
        #[arg(long)]
        naive_change: String,
        #[arg(long, default_value = "")]
        shortname: String,
        #[arg(long, default_value = "")]
        outcomes: String,
    },
    /// Write a defense persona markdown under defense-personas/.
    DefensePersona {
        #[arg(long)]
        name: String,
        #[arg(long)]
        body: String,
    },
    /// Write a defense strategy markdown under defense/strategy/.
    DefenseStrategy {
        #[arg(long)]
        name: String,
        #[arg(long)]
        body: String,
    },
    /// Write a defense progress markdown under defense/progress/.
    DefenseProgress {
        #[arg(long)]
        name: String,
        #[arg(long)]
        body: String,
    },
    /// Write a defense pitch markdown under defense/pitches/.
    DefensePitch {
        #[arg(long)]
        name: String,
        #[arg(long)]
        body: String,
    },
}

#[derive(Subcommand)]
pub enum RemoveTarget {
    /// Clear force×component coupling from residues.csv.
    Residue {
        #[arg(long)]
        force_id: String,
        #[arg(long)]
        component_id: String,
    },
    /// Remove a lexicon term by its canonical spelling.
    Term {
        #[arg(long)]
        term: String,
    },
}

#[derive(Subcommand)]
pub enum UpdateTarget {
    /// Update a stressor force in place; also folds residues.csv couplings.
    Stressor {
        #[arg(long)]
        force_id: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        attractor_id: Option<String>,
        #[arg(long)]
        naive_change: Option<String>,
        #[arg(long)]
        shortname: Option<String>,
        #[arg(long, visible_alias = "traits")]
        outcomes: Option<String>,
        /// Component id to couple to this stressor (repeatable).
        #[arg(long)]
        add_component: Vec<String>,
        /// Component id to decouple from this stressor (repeatable).
        #[arg(long)]
        remove_component: Vec<String>,
    },
    /// Update a purpose force in place; also folds residues.csv couplings.
    Purpose {
        #[arg(long)]
        force_id: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        attractor_id: Option<String>,
        #[arg(long, visible_alias = "feature")]
        naive_change: Option<String>,
        #[arg(long)]
        shortname: Option<String>,
        #[arg(long, visible_alias = "traits")]
        outcomes: Option<String>,
        /// Component id to couple to this purpose (repeatable).
        #[arg(long)]
        add_component: Vec<String>,
        /// Component id to decouple from this purpose (repeatable).
        #[arg(long)]
        remove_component: Vec<String>,
    },
    /// Update an attractor in place, keyed by its id (e.g. A-01).
    Attractor {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        positive_state: Option<String>,
        #[arg(long)]
        negative_state: Option<String>,
    },
    /// Update a component in place, keyed by its name.
    Component {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        architecture_set: Option<String>,
    },
    /// Update a lexicon term in place, keyed by its canonical spelling.
    Term {
        #[arg(long)]
        term: String,
        #[arg(long)]
        definition: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        related: Option<String>,
    },
    /// Update a persona in place, keyed by its name.
    Persona {
        #[arg(long)]
        name: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        concerns: Option<String>,
        #[arg(long)]
        desires: Option<String>,
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
    MetaStressors,
    MetaAttractors,
    MetaPurposes,
    DefensePersonas,
    DefenseStrategies,
    DefenseProgress,
    DefensePitches,
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
    Template { force_id: String },
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
        #[arg(long)]
        stressors: usize,
        #[arg(long)]
        naive_survived: usize,
        #[arg(long)]
        residual_survived: usize,
    },
    Fusion,
    Fission,
}

#[derive(Subcommand)]
pub enum TagOp {
    Scan {
        #[arg(default_value = ".")]
        path: String,
    },
    Report {
        #[arg(default_value = ".")]
        path: String,
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
        Command::Update { force, target } => crate::storage::update(&cfg, target, force),
        Command::Write { op } => match op {
            WriteOp::Authorize { minutes } => {
                let expires_at = crate::storage::write_authorization::authorize(&cfg, minutes)?;
                println!(
                    "Authorized sidecar metadata writes until {}. Do not stage {}.",
                    expires_at.to_rfc3339(),
                    crate::storage::write_authorization::marker_path(&cfg).display()
                );
                Ok(())
            }
        },
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
                print!(
                    "{}",
                    crate::verification::commit_msg::template_for_force(&cfg, &force_id)?
                );
                Ok(())
            }
        },
        Command::Matrix { op } => crate::structure::analysis::nkp::run(&cfg, op),
        Command::Skill { op } => match op {
            SkillCommand::Show { name, version } => crate::skills::phases::show(&name, version),
            SkillCommand::Data { name } => crate::skills::phases::data(&cfg, &name),
            SkillCommand::List => crate::skills::phases::list_all(),
            SkillCommand::Install {
                name,
                agent,
                global,
            } => crate::skills::installer::install(&name, &agent, global),
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
                // Deliberately not gated by write_authorization::require: walk stamps are
                // low-blast-radius cadence bookkeeping, not ledger content, and gating them
                // would force an explicit authorize step onto the routine reminder path.
                if completed {
                    walk_reminder::record_completed(&meta, walk_kind)?;
                    crate::storage::git_sidecar::persist_if_sidecar(&cfg, &meta)?;
                    println!("Recorded {}-walk completion", walk_kind.as_str());
                } else {
                    walk_reminder::record_deferred(&meta, walk_kind)?;
                    crate::storage::git_sidecar::persist_if_sidecar(&cfg, &meta)?;
                    println!("Recorded {}-walk deferral", walk_kind.as_str());
                }
                Ok(())
            }
        },
        Command::Hook { op } => match op {
            HookOp::PreCompaction => {
                println!("{}", crate::skills::hooks::pre_compaction());
                Ok(())
            }
            HookOp::PostCompaction => {
                println!("{}", crate::skills::hooks::post_compaction());
                Ok(())
            }
            HookOp::PeriodicTurn { turn, text } => {
                let text = match text {
                    Some(text) => text,
                    None => {
                        use std::io::Read;
                        let mut buf = String::new();
                        std::io::stdin()
                            .read_to_string(&mut buf)
                            .context("read discussion snippet from stdin")?;
                        buf
                    }
                };
                if let Some(suggestion) = crate::skills::hooks::periodic_turn(turn, &text)? {
                    println!("{suggestion}");
                }
                Ok(())
            }
        },
        Command::Branch { op } => run_branch(&cfg, op),
        Command::View { defense, out } => crate::view::run_view(&cfg, defense, out),
        Command::Serve { defense, port } => crate::view::run_serve(&cfg, defense, port),
        Command::Config => crate::config::print(&cfg),
    }
}

fn run_branch(cfg: &crate::config::Config, op: BranchOp) -> Result<()> {
    use crate::storage::git_sidecar;

    let sidecar = git_sidecar::SidecarConfig::from_config_file(&cfg.config_path)?;
    match op {
        BranchOp::Init { branch } => {
            let resolved = git_sidecar::branch_init(&cfg.repo_root, &sidecar, branch.as_deref())?;
            println!("Initialized metadata branch '{resolved}'");
            Ok(())
        }
        BranchOp::Edit => {
            crate::storage::write_authorization::require(cfg)?;
            let branch = git_sidecar::branch_edit(&cfg.repo_root, &sidecar, &cfg.config_host_dir)?;
            println!(
                "Materialized '{branch}' into {} for manual editing",
                cfg.config_host_dir.display()
            );
            Ok(())
        }
        BranchOp::Save { commit_msg, push } => {
            crate::storage::write_authorization::require(cfg)?;
            let branch = git_sidecar::branch_save(
                &cfg.repo_root,
                &sidecar,
                &cfg.config_host_dir,
                &commit_msg,
                push,
            )?;
            println!(
                "Saved edits to '{branch}'{}",
                if push { " and pushed" } else { "" }
            );
            Ok(())
        }
        BranchOp::Merge {
            from,
            to,
            commit_msg,
            push,
        } => {
            let (from_branch, to_branch) = git_sidecar::branch_merge(
                &cfg.repo_root,
                &sidecar,
                &from,
                &to,
                commit_msg.as_deref(),
                push,
            )?;
            println!(
                "Merged '{from_branch}' into '{to_branch}'{}",
                if push { " and pushed" } else { "" }
            );
            Ok(())
        }
        BranchOp::SyncCommit => {
            // Called from the post-commit hook — must never fail the commit.
            match git_sidecar::branch_sync_commit(&cfg.repo_root, &sidecar, &cfg.config_host_dir) {
                Ok(Some(branch)) => println!("Synced commit to '{branch}'"),
                Ok(None) => {}
                Err(e) => eprintln!("residual branch sync-commit: {e}"),
            }
            Ok(())
        }
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

    /// Phase 2: `residual add meta-stressor` with fields matching defense headers.
    #[test]
    fn cli_parses_add_meta_stressor() {
        let cli = Cli::try_parse_from([
            "residual",
            "add",
            "meta-stressor",
            "--shortname",
            "meta-force-landscape",
            "--description",
            "Defense-layer stressor",
        ]);
        assert!(
            cli.is_ok(),
            "CLI must accept `add meta-stressor` (id,shortname,description), got err: {}",
            cli.err().map(|e| e.to_string()).unwrap_or_default()
        );
    }

    #[test]
    fn cli_parses_add_meta_attractor() {
        let cli = Cli::try_parse_from([
            "residual",
            "add",
            "meta-attractor",
            "--name",
            "Practitioner Standing",
            "--description",
            "Defense attractor",
            "--positive-state",
            "safe to discuss",
            "--negative-state",
            "reads as heresy",
        ]);
        assert!(
            cli.is_ok(),
            "CLI must accept `add meta-attractor` matching meta-attractors.csv header, got err: {}",
            cli.err().map(|e| e.to_string()).unwrap_or_default()
        );
    }

    #[test]
    fn cli_parses_add_meta_purpose() {
        let cli = Cli::try_parse_from([
            "residual",
            "add",
            "meta-purpose",
            "--shortname",
            "defense-efficacy",
            "--description",
            "Defense purpose",
            "--naive-change",
            "capture meta forces",
            "--outcomes",
            "operator records defense purpose",
            "--attractor-id",
            "MA-01",
        ]);
        assert!(
            cli.is_ok(),
            "CLI must accept `add meta-purpose` matching meta-purposes.csv header, got err: {}",
            cli.err().map(|e| e.to_string()).unwrap_or_default()
        );
    }

    #[test]
    fn cli_parses_add_defense_persona_strategy_progress_pitch() {
        for (sub, name) in [
            ("defense-persona", "hostile-auditor"),
            ("defense-strategy", "alpha"),
            ("defense-progress", "week-1"),
            ("defense-pitch", "exec-summary"),
        ] {
            let cli =
                Cli::try_parse_from(["residual", "add", sub, "--name", name, "--body", "# md\n"]);
            assert!(
                cli.is_ok(),
                "CLI must accept `add {sub}`, got err: {}",
                cli.err().map(|e| e.to_string()).unwrap_or_default()
            );
        }
    }

    #[test]
    fn cli_parses_list_defense_meta_and_artifacts() {
        for target in [
            "meta-stressors",
            "meta-attractors",
            "meta-purposes",
            "defense-personas",
            "defense-strategies",
            "defense-progress",
            "defense-pitches",
        ] {
            let cli = Cli::try_parse_from(["residual", "list", target]);
            assert!(
                cli.is_ok(),
                "CLI must accept `list {target}`, got err: {}",
                cli.err().map(|e| e.to_string()).unwrap_or_default()
            );
        }
    }

    #[test]
    fn add_purpose_accepts_naive_change_flag() {
        let cli = Cli::try_parse_from([
            "residual",
            "add",
            "purpose",
            "--description",
            "d",
            "--attractor-id",
            "A-01",
            "--naive-change",
            "naive change text",
        ])
        .unwrap();
        match cli.command {
            Command::Add {
                target: AddTarget::Purpose { naive_change, .. },
                ..
            } => {
                assert_eq!(naive_change, "naive change text");
            }
            _ => panic!("expected Command::Add(AddTarget::Purpose)"),
        }
    }

    #[test]
    fn add_purpose_feature_alias_maps_to_naive_change() {
        let cli = Cli::try_parse_from([
            "residual",
            "add",
            "purpose",
            "--description",
            "d",
            "--attractor-id",
            "A-01",
            "--feature",
            "aliased text",
        ])
        .unwrap();
        match cli.command {
            Command::Add {
                target: AddTarget::Purpose { naive_change, .. },
                ..
            } => {
                assert_eq!(naive_change, "aliased text");
            }
            _ => panic!("expected Command::Add(AddTarget::Purpose)"),
        }
    }

    #[test]
    fn branch_merge_parses_from_to_and_flags() {
        let cli = Cli::try_parse_from([
            "residual",
            "branch",
            "merge",
            "feature/x",
            "main",
            "--commit-msg",
            "fold it in",
            "--push",
        ])
        .unwrap();
        match cli.command {
            Command::Branch {
                op:
                    BranchOp::Merge {
                        from,
                        to,
                        commit_msg,
                        push,
                    },
            } => {
                assert_eq!(from, "feature/x");
                assert_eq!(to, "main");
                assert_eq!(commit_msg.as_deref(), Some("fold it in"));
                assert!(push);
            }
            _ => panic!("expected Command::Branch(BranchOp::Merge)"),
        }
    }

    #[test]
    fn cli_parses_view_with_defense_flag() {
        let cli = Cli::try_parse_from(["residual", "view", "--defense"])
            .expect("CLI must accept `view --defense`");
        match cli.command {
            Command::View { defense, out } => {
                assert!(defense, "--defense must set the defense flag");
                assert_eq!(out, None, "--out defaults to a temp dir");
            }
            _ => panic!("expected Command::View"),
        }
    }

    #[test]
    fn cli_view_defaults_defense_off() {
        let cli = Cli::try_parse_from(["residual", "view"]).expect("CLI must accept bare `view`");
        match cli.command {
            Command::View { defense, .. } => {
                assert!(!defense, "defense must default to OFF");
            }
            _ => panic!("expected Command::View"),
        }
    }

    #[test]
    fn cli_parses_view_out_dir() {
        let cli = Cli::try_parse_from(["residual", "view", "--out", "/tmp/landscape"])
            .expect("CLI must accept `view --out <dir>`");
        match cli.command {
            Command::View { out, .. } => {
                assert_eq!(out.as_deref(), Some(std::path::Path::new("/tmp/landscape")));
            }
            _ => panic!("expected Command::View"),
        }
    }

    #[test]
    fn cli_parses_serve_with_port() {
        let cli = Cli::try_parse_from(["residual", "serve", "--port", "8765"])
            .expect("CLI must accept `serve --port <u16>`");
        match cli.command {
            Command::Serve { defense, port } => {
                assert_eq!(port, 8765);
                assert!(!defense, "defense must default to OFF");
            }
            _ => panic!("expected Command::Serve"),
        }
    }

    #[test]
    fn cli_serve_defaults_to_view_default_port() {
        let cli = Cli::try_parse_from(["residual", "serve", "--defense"])
            .expect("CLI must accept `serve --defense`");
        match cli.command {
            Command::Serve { defense, port } => {
                assert!(defense);
                assert_eq!(port, crate::view::DEFAULT_PORT);
            }
            _ => panic!("expected Command::Serve"),
        }
    }

    #[test]
    fn branch_init_branch_flag_defaults_to_none() {
        let cli = Cli::try_parse_from(["residual", "branch", "init"]).unwrap();
        match cli.command {
            Command::Branch {
                op: BranchOp::Init { branch },
            } => {
                assert_eq!(branch, None);
            }
            _ => panic!("expected Command::Branch(BranchOp::Init)"),
        }
    }

    #[test]
    fn cli_parses_update_stressor_with_only_identity_flag() {
        let cli = Cli::try_parse_from(["residual", "update", "stressor", "--force-id", "S-01"]);
        assert!(
            cli.is_ok(),
            "CLI must accept `update stressor` with only --force-id (all other fields optional), got err: {}",
            cli.err().map(|e| e.to_string()).unwrap_or_default()
        );
        match cli.unwrap().command {
            Command::Update {
                target:
                    UpdateTarget::Stressor {
                        force_id,
                        description,
                        attractor_id,
                        naive_change,
                        shortname,
                        outcomes,
                        add_component,
                        remove_component,
                    },
                ..
            } => {
                assert_eq!(force_id, "S-01");
                assert_eq!(description, None);
                assert_eq!(attractor_id, None);
                assert_eq!(naive_change, None);
                assert_eq!(shortname, None);
                assert_eq!(outcomes, None);
                assert!(add_component.is_empty());
                assert!(remove_component.is_empty());
            }
            _ => panic!("expected Command::Update(UpdateTarget::Stressor)"),
        }
    }

    #[test]
    fn cli_parses_update_stressor_with_repeated_component_flags() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "stressor",
            "--force-id",
            "S-01",
            "--description",
            "new desc",
            "--add-component",
            "auth",
            "--add-component",
            "db",
            "--remove-component",
            "legacy",
        ])
        .expect("CLI must accept repeated --add-component/--remove-component on update stressor");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Stressor {
                        description,
                        add_component,
                        remove_component,
                        ..
                    },
                ..
            } => {
                assert_eq!(description.as_deref(), Some("new desc"));
                assert_eq!(add_component, vec!["auth".to_string(), "db".to_string()]);
                assert_eq!(remove_component, vec!["legacy".to_string()]);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Stressor)"),
        }
    }

    #[test]
    fn cli_parses_update_purpose_with_component_flags() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "purpose",
            "--force-id",
            "P-01",
            "--naive-change",
            "revised feature",
            "--add-component",
            "auth",
        ])
        .expect("CLI must accept `update purpose` with optional fields + --add-component");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Purpose {
                        force_id,
                        naive_change,
                        description,
                        add_component,
                        ..
                    },
                ..
            } => {
                assert_eq!(force_id, "P-01");
                assert_eq!(naive_change.as_deref(), Some("revised feature"));
                assert_eq!(description, None);
                assert_eq!(add_component, vec!["auth".to_string()]);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Purpose)"),
        }
    }

    #[test]
    fn cli_parses_update_attractor_keyed_by_id() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "attractor",
            "--id",
            "A-01",
            "--positive-state",
            "new positive",
        ])
        .expect("CLI must accept `update attractor` keyed by --id with optional fields");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Attractor {
                        id,
                        positive_state,
                        name,
                        description,
                        negative_state,
                    },
                ..
            } => {
                assert_eq!(id, "A-01");
                assert_eq!(positive_state.as_deref(), Some("new positive"));
                assert_eq!(name, None);
                assert_eq!(description, None);
                assert_eq!(negative_state, None);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Attractor)"),
        }
    }

    #[test]
    fn cli_parses_update_component_keyed_by_name() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "component",
            "--name",
            "auth",
            "--status",
            "actual",
        ])
        .expect("CLI must accept `update component` keyed by --name with optional fields");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Component {
                        name,
                        status,
                        description,
                        architecture_set,
                    },
                ..
            } => {
                assert_eq!(name, "auth");
                assert_eq!(status.as_deref(), Some("actual"));
                assert_eq!(description, None);
                assert_eq!(architecture_set, None);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Component)"),
        }
    }

    #[test]
    fn cli_parses_update_term_keyed_by_term() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "term",
            "--term",
            "attractor",
            "--definition",
            "new definition",
        ])
        .expect("CLI must accept `update term` keyed by --term with optional fields");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Term {
                        term,
                        definition,
                        domain,
                        related,
                    },
                ..
            } => {
                assert_eq!(term, "attractor");
                assert_eq!(definition.as_deref(), Some("new definition"));
                assert_eq!(domain, None);
                assert_eq!(related, None);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Term)"),
        }
    }

    #[test]
    fn cli_parses_update_persona_keyed_by_name() {
        let cli = Cli::try_parse_from([
            "residual",
            "update",
            "persona",
            "--name",
            "alice",
            "--role",
            "lead engineer",
        ])
        .expect("CLI must accept `update persona` keyed by --name with optional fields");
        match cli.command {
            Command::Update {
                target:
                    UpdateTarget::Persona {
                        name,
                        role,
                        concerns,
                        desires,
                    },
                ..
            } => {
                assert_eq!(name, "alice");
                assert_eq!(role.as_deref(), Some("lead engineer"));
                assert_eq!(concerns, None);
                assert_eq!(desires, None);
            }
            _ => panic!("expected Command::Update(UpdateTarget::Persona)"),
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
            let cfg = Config::for_test_residual_dir(&residual);
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

            let residues =
                std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
            assert!(
                !residues.contains(",1")
                    || !residues
                        .lines()
                        .any(|l| l.starts_with("S-01,") && l.contains("1")),
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

            let residues =
                std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
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

        fn add_attractor(dir: &tempfile::TempDir) {
            let status = run(
                dir,
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
            assert!(
                status.status.success(),
                "add attractor: {}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        fn add_stressor(dir: &tempfile::TempDir, description: &str, naive_change: &str) {
            let status = run(
                dir,
                &[
                    "add",
                    "stressor",
                    "--description",
                    description,
                    "--attractor-id",
                    "A-01",
                    "--naive-change",
                    naive_change,
                ],
            );
            assert!(
                status.status.success(),
                "add stressor: {}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        fn add_purpose(dir: &tempfile::TempDir, description: &str, naive_change: &str) {
            let status = run(
                dir,
                &[
                    "add",
                    "purpose",
                    "--description",
                    description,
                    "--attractor-id",
                    "A-01",
                    "--naive-change",
                    naive_change,
                ],
            );
            assert!(
                status.status.success(),
                "add purpose: {}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        // --- residual update <type>: red-phase tests. All storage::*::update()
        // bodies are `todo!()` stubs (green phase implements them), so every
        // test below currently fails via panic (non-zero exit / stderr
        // "not implemented"), not via compile error.

        #[test]
        fn update_stressor_changes_description_leaves_naive_change() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);
            add_stressor(&dir, "original desc", "original naive change");

            let update = run(
                &dir,
                &[
                    "update",
                    "stressor",
                    "--force-id",
                    "S-01",
                    "--description",
                    "revised desc",
                ],
            );
            assert!(
                update.status.success(),
                "update stressor --description: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let stressors =
                std::fs::read_to_string(dir.path().join("residual/stressors.csv")).unwrap();
            let row = stressors
                .lines()
                .find(|l| l.starts_with("S-01,"))
                .expect("S-01 row");
            assert!(
                row.contains("revised desc"),
                "description must be updated, row={row}"
            );
            assert!(
                row.contains("original naive change"),
                "naive_change must be left unchanged when omitted, row={row}"
            );
        }

        #[test]
        fn update_stressor_unknown_id_errors_with_no_partial_write() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);
            add_stressor(&dir, "original desc", "original naive change");
            let before =
                std::fs::read_to_string(dir.path().join("residual/stressors.csv")).unwrap();

            let update = run(
                &dir,
                &[
                    "update",
                    "stressor",
                    "--force-id",
                    "S-99",
                    "--description",
                    "should not land",
                ],
            );
            assert!(
                !update.status.success(),
                "update stressor with unknown --force-id must error"
            );

            let after =
                std::fs::read_to_string(dir.path().join("residual/stressors.csv")).unwrap();
            assert_eq!(
                before, after,
                "unknown-id update must not partially write stressors.csv"
            );
        }

        #[test]
        fn update_stressor_add_component_couples_residue() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_component(&dir, "auth", "Auth");
            add_attractor(&dir);
            add_stressor(&dir, "load", "cache");

            let update = run(
                &dir,
                &["update", "stressor", "--force-id", "S-01", "--add-component", "auth"],
            );
            assert!(
                update.status.success(),
                "update stressor --add-component: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let residues =
                std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
            let header = residues.lines().next().expect("header");
            let cols: Vec<&str> = header.split(',').map(str::trim).collect();
            let row = residues
                .lines()
                .find(|l| l.starts_with("S-01,"))
                .expect("S-01 row");
            let cells: Vec<&str> = row.split(',').map(str::trim).collect();
            let auth_idx = cols.iter().position(|c| *c == "auth").expect("auth column");
            assert_eq!(
                cells.get(auth_idx).copied().unwrap_or(""),
                "1",
                "auth coupling must be set via --add-component, row={row}"
            );
        }

        #[test]
        fn update_stressor_remove_component_decouples_residue() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_component(&dir, "auth", "Auth");
            // The residues matrix (src/storage/format.rs::residues_to_rows) only
            // emits a row for a force when it has at least one *coupled* residue
            // (see remove_residue_clears_matrix_cell for the same pattern). Seed a
            // second, unrelated component coupling so the S-01 row still exists
            // after --remove-component clears the "auth" cell — otherwise the row
            // itself would legitimately disappear, which is not what this test is
            // exercising.
            add_component(&dir, "db", "Database");
            add_attractor(&dir);
            add_stressor(&dir, "load", "cache");
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
            run(
                &dir,
                &[
                    "add",
                    "residue",
                    "--force-id",
                    "S-01",
                    "--component-id",
                    "db",
                ],
            );

            let update = run(
                &dir,
                &[
                    "update",
                    "stressor",
                    "--force-id",
                    "S-01",
                    "--remove-component",
                    "auth",
                ],
            );
            assert!(
                update.status.success(),
                "update stressor --remove-component: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let residues =
                std::fs::read_to_string(dir.path().join("residual/residues.csv")).unwrap();
            let header = residues.lines().next().expect("header");
            let cols: Vec<&str> = header.split(',').map(str::trim).collect();
            let row = residues
                .lines()
                .find(|l| l.starts_with("S-01,"))
                .expect("S-01 row");
            let cells: Vec<&str> = row.split(',').map(str::trim).collect();
            let auth_idx = cols.iter().position(|c| *c == "auth").expect("auth column");
            assert_ne!(
                cells.get(auth_idx).copied().unwrap_or(""),
                "1",
                "auth coupling must be cleared via --remove-component, row={row}"
            );
        }

        #[test]
        fn update_purpose_changes_naive_change_leaves_description() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);
            add_purpose(&dir, "original purpose desc", "original feature");

            let update = run(
                &dir,
                &[
                    "update",
                    "purpose",
                    "--force-id",
                    "P-01",
                    "--naive-change",
                    "revised feature",
                ],
            );
            assert!(
                update.status.success(),
                "update purpose --naive-change: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let purposes =
                std::fs::read_to_string(dir.path().join("residual/purposes.csv")).unwrap();
            let row = purposes
                .lines()
                .find(|l| l.starts_with("P-01,"))
                .expect("P-01 row");
            assert!(
                row.contains("revised feature"),
                "naive_change must be updated, row={row}"
            );
            assert!(
                row.contains("original purpose desc"),
                "description must be left unchanged when omitted, row={row}"
            );
        }

        #[test]
        fn update_purpose_unknown_id_errors() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);
            add_purpose(&dir, "desc", "feature");

            let update = run(
                &dir,
                &[
                    "update",
                    "purpose",
                    "--force-id",
                    "P-99",
                    "--naive-change",
                    "nope",
                ],
            );
            assert!(
                !update.status.success(),
                "update purpose with unknown --force-id must error"
            );
        }

        #[test]
        fn update_attractor_changes_positive_state_leaves_name() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);

            let update = run(
                &dir,
                &[
                    "update",
                    "attractor",
                    "--id",
                    "A-01",
                    "--positive-state",
                    "new positive",
                ],
            );
            assert!(
                update.status.success(),
                "update attractor --positive-state: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let attractors =
                std::fs::read_to_string(dir.path().join("residual/attractors.csv")).unwrap();
            let row = attractors
                .lines()
                .find(|l| l.starts_with("A-01,"))
                .expect("A-01 row");
            assert!(
                row.contains("new positive"),
                "positive_state must be updated, row={row}"
            );
            assert!(
                row.contains("X"),
                "name must be left unchanged when omitted, row={row}"
            );
        }

        #[test]
        fn update_attractor_unknown_id_errors() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_attractor(&dir);

            let update = run(
                &dir,
                &[
                    "update",
                    "attractor",
                    "--id",
                    "A-99",
                    "--positive-state",
                    "nope",
                ],
            );
            assert!(
                !update.status.success(),
                "update attractor with unknown --id must error"
            );
        }

        #[test]
        fn update_component_changes_status_leaves_description() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            add_component(&dir, "auth", "Auth module description");

            let update = run(
                &dir,
                &["update", "component", "--name", "auth", "--status", "actual"],
            );
            assert!(
                update.status.success(),
                "update component --status: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let components =
                std::fs::read_to_string(dir.path().join("residual/components.csv")).unwrap();
            let row = components
                .lines()
                .find(|l| l.starts_with("auth,"))
                .expect("auth row");
            assert!(
                row.contains("actual"),
                "status must be updated, row={row}"
            );
            assert!(
                row.contains("Auth module description"),
                "description must be left unchanged when omitted, row={row}"
            );
        }

        #[test]
        fn update_component_unknown_name_errors() {
            let dir = isolated_tempdir();
            init_storage(&dir);

            let update = run(
                &dir,
                &[
                    "update",
                    "component",
                    "--name",
                    "does-not-exist",
                    "--status",
                    "actual",
                ],
            );
            assert!(
                !update.status.success(),
                "update component with unknown --name must error"
            );
        }

        #[test]
        fn update_term_changes_definition_leaves_domain() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            let added = run(
                &dir,
                &[
                    "add",
                    "term",
                    "--term",
                    "hyperliminal",
                    "--definition",
                    "original definition",
                    "--domain",
                    "coupling",
                ],
            );
            assert!(
                added.status.success(),
                "add term: {}",
                String::from_utf8_lossy(&added.stderr)
            );

            let update = run(
                &dir,
                &[
                    "update",
                    "term",
                    "--term",
                    "hyperliminal",
                    "--definition",
                    "revised definition",
                ],
            );
            assert!(
                update.status.success(),
                "update term --definition: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let lexicon =
                std::fs::read_to_string(dir.path().join("residual/lexicon.csv")).unwrap();
            let row = lexicon
                .lines()
                .find(|l| l.starts_with("hyperliminal,"))
                .expect("hyperliminal row");
            assert!(
                row.contains("revised definition"),
                "definition must be updated, row={row}"
            );
            assert!(
                row.contains("coupling"),
                "domain must be left unchanged when omitted, row={row}"
            );
        }

        #[test]
        fn update_term_unknown_term_errors() {
            let dir = isolated_tempdir();
            init_storage(&dir);

            let update = run(
                &dir,
                &[
                    "update",
                    "term",
                    "--term",
                    "does-not-exist",
                    "--definition",
                    "nope",
                ],
            );
            assert!(
                !update.status.success(),
                "update term with unknown --term must error"
            );
        }

        #[test]
        fn update_persona_changes_role_leaves_desires() {
            let dir = isolated_tempdir();
            init_storage(&dir);
            let added = run(
                &dir,
                &[
                    "add",
                    "persona",
                    "--name",
                    "alice",
                    "--role",
                    "original role",
                    "--desires",
                    "reliability",
                ],
            );
            assert!(
                added.status.success(),
                "add persona: {}",
                String::from_utf8_lossy(&added.stderr)
            );

            let update = run(
                &dir,
                &[
                    "update",
                    "persona",
                    "--name",
                    "alice",
                    "--role",
                    "revised role",
                ],
            );
            assert!(
                update.status.success(),
                "update persona --role: {}",
                String::from_utf8_lossy(&update.stderr)
            );

            let persona =
                std::fs::read_to_string(dir.path().join("residual/personas/alice.md")).unwrap();
            assert!(
                persona.contains("revised role"),
                "role must be updated, got: {persona}"
            );
            assert!(
                persona.contains("reliability"),
                "desires must be left unchanged when omitted, got: {persona}"
            );
        }

        #[test]
        fn update_persona_unknown_name_errors() {
            let dir = isolated_tempdir();
            init_storage(&dir);

            let update = run(
                &dir,
                &["update", "persona", "--name", "does-not-exist", "--role", "nope"],
            );
            assert!(
                !update.status.success(),
                "update persona with unknown --name must error"
            );
        }
    }
}
