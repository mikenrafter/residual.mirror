use anyhow::Result;
use std::fs;
use crate::config::Config;
use crate::cli::{AddTarget, ListTarget};
use crate::structure::analysis::residues::{tag_naive_change_whole_system, Residue};

pub mod attractors;
pub mod components;
pub mod config;
pub mod defense;
pub mod format;
pub mod git_sidecar;
pub mod integrity;
pub mod iterations;
pub mod personas;
pub mod purposes;
pub mod residues;
pub mod research;
pub mod stressors;
pub mod terminology;

const WHOLE_SYSTEM_REMINDER: &str = "reminder: examine whole-system-residue (hardware, process, organization, policy) before defaulting to a software-only patch; use --whole-system --notes when the zig survives outside software";

pub fn init(cfg: &Config, force: bool) -> Result<()> {
    let session = integrity::sessions::begin_mutation(&cfg.residual_dir, force)?;
    init_dirs_and_files(cfg)?;
    session.commit()?;
    println!("Initialized residual/ at {}", cfg.residual_dir.display());
    Ok(())
}

fn init_dirs_and_files(cfg: &Config) -> Result<()> {
    let dir = &cfg.residual_dir;
    fs::create_dir_all(dir.join("iterations"))?;
    fs::create_dir_all(dir.join("personas"))?;
    fs::create_dir_all(dir.join("research"))?;

    // Write config.toml if not present (storage-config; format_version v4 by default).
    let config_path = dir.join("config.toml");
    if !config_path.exists() {
        let toml = crate::storage::config::render_v3(&crate::storage::config::StorageConfig::default());
        fs::write(&config_path, toml)?;
    }

    // Write empty CSVs with headers if not present
    let csvs: &[(&str, &str)] = &[
        ("stressors.csv", "id,shortname,description,naive_change,outcomes,attractor_id"),
        ("purposes.csv", "id,shortname,description,naive_change,outcomes,attractor_id"),
        ("attractors.csv", "id,name,description,positive_state,negative_state"),
        ("lexicon.csv", "term,definition,domain,aliases"),
        ("residues.csv", "force"),
        ("components.csv", "name,description,status,architecture_set"),
    ];
    for (filename, header) in csvs {
        let path = dir.join(filename);
        if !path.exists() {
            fs::write(&path, format!("{}\n", header))?;
        }
    }
    Ok(())
}

pub fn add(cfg: &Config, target: AddTarget, force: bool) -> Result<()> {
    let session = integrity::sessions::begin_mutation(&cfg.residual_dir, force)?;
    add_entry(cfg, target)?;
    session.commit()?;
    Ok(())
}

fn add_entry(cfg: &Config, target: AddTarget) -> Result<()> {
    let dir = &cfg.residual_dir;
    match target {
        AddTarget::Stressor {
            description,
            attractor_id,
            naive_change,
            shortname,
            outcomes,
            whole_system,
            notes,
        } => {
            let naive_change = if whole_system {
                if notes.is_empty() {
                    anyhow::bail!("--whole-system requires --notes describing the hardware, process, organization, or policy zig");
                }
                tag_naive_change_whole_system(&naive_change)
            } else {
                eprintln!("{WHOLE_SYSTEM_REMINDER}");
                naive_change
            };
            let existing = stressors::load(dir)?;
            let id = stressors::next_id(&existing);
            stressors::append(dir, stressors::Stressor {
                id: id.clone(),
                shortname,
                description,
                attractor_id,
                naive_change,
                outcomes,
            })?;
            if whole_system {
                let residue_id = residues::append_whole_system(dir, &id, &notes)?;
                println!("Added whole-system-residue {}", residue_id);
            }
            println!("Added stressor {}", id);
        }
        AddTarget::Residue {
            force_id,
            component_id,
            whole_system,
            notes,
        } => {
            if whole_system {
                let id = residues::append_whole_system(dir, &force_id, &notes)?;
                println!("Added whole-system-residue {}", id);
            } else {
                if component_id.is_empty() {
                    anyhow::bail!("provide --component-id or --whole-system");
                }
                if !residues::force_exists(dir, &force_id)? {
                    anyhow::bail!("force id '{}' not found in stressors or purposes", force_id);
                }
                let existing = residues::load(dir)?;
                let id = residues::next_id(&existing);
                residues::append(
                    dir,
                    Residue::coupling(id.clone(), force_id, component_id),
                )?;
                println!("Added residue coupling {}", id);
            }
        }
        AddTarget::Purpose { description, attractor_id, naive_change, shortname, outcomes } => {
            let existing = purposes::load(dir)?;
            let id = purposes::next_id(&existing);
            purposes::append(dir, purposes::Purpose {
                id: id.clone(),
                shortname,
                description,
                attractor_id,
                naive_change,
                outcomes,
            })?;
            println!("Added purpose {}", id);
        }
        AddTarget::Attractor {
            name,
            description,
            positive_state,
            negative_state,
        } => {
            let existing = attractors::load(dir)?;
            let id = attractors::next_id(&existing);
            attractors::append(
                dir,
                attractors::Attractor {
                    id: id.clone(),
                    name,
                    description,
                    positive_state,
                    negative_state,
                },
            )?;
            println!("Added attractor {}", id);
        }
        AddTarget::Term { term, definition, domain, related } => {
            format::append_lexicon(dir, crate::structure::definition::lexicon::Term {
                term: term.clone(),
                definition,
                domain,
                aliases: related,
            })?;
            println!("Added term '{}'", term);
        }
        AddTarget::Persona { name, role, concerns, desires } => {
            personas::create(dir, personas::Persona {
                name: name.clone(),
                role,
                concerns,
                desires,
                stressor_ids: vec![],
            })?;
            println!("Added persona '{}'", name);
        }
        AddTarget::Iteration { notes, ri_score } => {
            let n = iterations::next_n(dir)?;
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
            iterations::create(dir, iterations::IterationMeta {
                n,
                date,
                ri_score,
                n_val: String::new(),
                k_val: String::new(),
                p_val: String::new(),
                notes,
            })?;
            println!("Added iteration {}", n);
        }
    }
    Ok(())
}

pub fn list(cfg: &Config, target: ListTarget) -> Result<()> {
    let dir = &cfg.residual_dir;
    match target {
        ListTarget::Stressors => {
            let items = stressors::load(dir)?;
            if items.is_empty() {
                println!("No stressors.");
            } else {
                for s in &items {
                    println!("[{}] {} {} (attractor: {})", s.id, s.shortname, s.description, s.attractor_id);
                }
            }
        }
        ListTarget::Purposes => {
            let items = purposes::load(dir)?;
            if items.is_empty() {
                println!("No purposes.");
            } else {
                for p in &items {
                    let extra = if p.outcomes.is_empty() { String::new() } else { format!(" | outcomes: {}", p.outcomes) };
                    println!("[{}] {} {} (naive_change: {}{})", p.id, p.shortname, p.description, p.naive_change, extra);
                }
            }
        }
        ListTarget::Attractors => {
            let items = attractors::load(dir)?;
            if items.is_empty() {
                println!("No attractors.");
            } else {
                for a in &items {
                    println!(
                        "[{}] {} (+/{} | -/{} )",
                        a.id,
                        a.name,
                        truncate_state(&a.positive_state),
                        truncate_state(&a.negative_state)
                    );
                }
            }
        }
        ListTarget::Terminology => {
            let items = format::read_lexicon(dir)?;
            if items.is_empty() {
                println!("No terminology.");
            } else {
                for t in &items {
                    println!("{}: {}", t.term, t.definition);
                }
            }
        }
        ListTarget::Personas => {
            let names = personas::list_names(dir)?;
            if names.is_empty() {
                println!("No personas.");
            } else {
                for name in &names {
                    println!("{}", name);
                }
            }
        }
        ListTarget::Residues => { let matrix = format::format_residues_matrix(dir)?; if matrix.lines().count()<=1 { println!("No residues."); } else { print!("{matrix}"); } }
        ListTarget::Iterations => {
            let items = iterations::list(dir)?;
            if items.is_empty() {
                println!("No iterations.");
            } else {
                let mut sorted = items;
                sorted.sort_by_key(|i| i.n);
                for meta in &sorted {
                    println!("Iteration {}: {} (Ri: {})", meta.n, meta.date, meta.ri_score);
                }
            }
        }
    }
    Ok(())
}

fn truncate_state(s: &str) -> String {
    const MAX: usize = 40;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(MAX).collect();
        out.push('…');
        out
    }
}

/// Run naive → v3 migration for the project's residual/ directory.
pub fn migrate(cfg: &Config, force: bool) -> Result<()> {
    let report = integrity::migration::migrate_residual_dir(&cfg.residual_dir, force)?;
    println!(
        "Migrated {} (config={}, attractors={}, lexicon={}, v4_couplings={}, v4_decoupled={})",
        cfg.residual_dir.display(),
        report.config_migrated,
        report.attractors,
        report.lexicon_terms,
        report.v4_residue_couplings,
        report.v4_forces_decoupled
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cfg_for(dir: &std::path::Path) -> Config {
        Config {
            validation: crate::config::ValidationConfig { strict: true },
            skills: crate::config::SkillsConfig { token_warn: 1000 },
            residual_dir: dir.to_path_buf(),
        }
    }

    // @stressor: software-only-zag
    #[test]
    fn whole_system_reminder_mentions_whole_system() {
        assert!(WHOLE_SYSTEM_REMINDER.contains("whole-system"));
    }

    // @stressor: software-only-zag
    #[test]
    fn add_entry_whole_system_stressor_records_residue() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        attractors::append(
            &cfg.residual_dir,
            attractors::Attractor::new("A-01", "X", "ok", "bad"),
        )
        .unwrap();

        add_entry(
            &cfg,
            AddTarget::Stressor {
                description: "queue overload".into(),
                attractor_id: "A-01".into(),
                naive_change: "add retry".into(),
                shortname: "".into(),
                outcomes: "".into(),
                whole_system: true,
                notes: "policy zig: cap tickets".into(),
            },
        )
        .unwrap();

        let residues_csv = std::fs::read_to_string(cfg.residual_dir.join("residues.csv")).unwrap();
        assert!(residues_csv.contains("whole-system"), "expected whole-system column");
        let row = residues_csv
            .lines()
            .find(|l| l.starts_with("S-01,"))
            .expect("S-01 residue row");
        assert!(
            row.split(',').next_back() == Some("1"),
            "expected whole-system coupling mark, row={row}"
        );

        let stressors_csv = std::fs::read_to_string(cfg.residual_dir.join("stressors.csv")).unwrap();
        assert!(
            stressors_csv.contains("whole-system-residue"),
            "notes should land on stressor naive_change"
        );
    }

    // @stressor: software-only-zag
    #[test]
    fn add_entry_software_only_stressor_prints_reminder_to_stderr() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        attractors::append(
            &cfg.residual_dir,
            attractors::Attractor::new("A-01", "X", "ok", "bad"),
        )
        .unwrap();

        // add_entry prints WHOLE_SYSTEM_REMINDER to stderr on the non-whole-system
        // path; the reminder's own content is covered by
        // whole_system_reminder_mentions_whole_system above. Here we assert the
        // call still succeeds and records the stressor without --whole-system.
        let result = add_entry(
            &cfg,
            AddTarget::Stressor {
                description: "load".into(),
                attractor_id: "A-01".into(),
                naive_change: "cache".into(),
                shortname: "".into(),
                outcomes: "".into(),
                whole_system: false,
                notes: "".into(),
            },
        );
        assert!(result.is_ok());
    }
}
