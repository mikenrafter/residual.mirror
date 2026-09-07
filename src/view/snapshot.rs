//! Landscape snapshot — merged read model for `residual view` / `residual serve`.
//!
//! Two sources feed a snapshot: the sidecar tip metadata (`storage::metadata_dir`)
//! and the capture-session working tree (`config_host_dir` after `branch edit`, or
//! the inline residual/ dir). Working records win on id collision so an operator
//! sees uncommitted capture-session work in the landscape.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::storage::{
    attractors, components, defense, purposes, residues, stressors,
};

/// Which on-disk ledger a record came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordSource {
    /// Sidecar tip metadata (committed).
    Sidecar,
    /// Capture-session / working metadata (uncommitted).
    Working,
}

/// Stressor vs purpose, for force-list filtering in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForceKind {
    Stressor,
    Purpose,
}

/// Resolved read sources for a snapshot. `working_dir` is `None` when the
/// capture-session dir is absent or identical to the sidecar dir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandscapeSources {
    pub sidecar_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotForce {
    pub id: String,
    pub shortname: String,
    pub description: String,
    pub naive_change: String,
    pub outcomes: String,
    pub attractor_id: String,
    pub kind: ForceKind,
    pub source: RecordSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAttractor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub positive_state: String,
    pub negative_state: String,
    pub source: RecordSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotComponent {
    pub name: String,
    pub description: String,
    pub status: String,
    pub architecture_set: String,
    pub source: RecordSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotResidue {
    pub force_id: String,
    pub component_id: String,
    pub coupled: bool,
    pub whole_system: bool,
    pub notes: String,
    pub source: RecordSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMetaForce {
    pub id: String,
    pub shortname: String,
    pub description: String,
    pub naive_change: String,
    pub outcomes: String,
    pub attractor_id: String,
    pub kind: ForceKind,
    pub source: RecordSource,
}

/// Defense ledger projection. Present only when `include_defense` is true.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefenseSection {
    pub meta_stressors: Vec<SnapshotMetaForce>,
    pub meta_purposes: Vec<SnapshotMetaForce>,
    pub meta_attractors: Vec<SnapshotAttractor>,
    pub personas: Vec<String>,
    pub strategies: Vec<String>,
    pub progress: Vec<String>,
    pub pitches: Vec<String>,
}

/// Everything the HTML landscape renders from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandscapeSnapshot {
    pub attractors: Vec<SnapshotAttractor>,
    pub stressors: Vec<SnapshotForce>,
    pub purposes: Vec<SnapshotForce>,
    pub components: Vec<SnapshotComponent>,
    pub residues: Vec<SnapshotResidue>,
    /// Omitted entirely (no JSON key) unless the snapshot was loaded with `--defense`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defense: Option<DefenseSection>,
}

impl LandscapeSnapshot {
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

}

/// Resolve sidecar + capture-session dirs from a loaded config.
pub fn resolve_sources(cfg: &Config) -> Result<LandscapeSources> {
    let sidecar_dir = crate::storage::metadata_dir(cfg)?;
    let working = &cfg.config_host_dir;
    let working_dir = if working != &sidecar_dir && working.is_dir() {
        Some(working.clone())
    } else {
        None
    };
    Ok(LandscapeSources {
        sidecar_dir: Some(sidecar_dir),
        working_dir,
    })
}

/// Load and merge a landscape snapshot from explicit directories (pure, git-free).
pub fn load_landscape_snapshot_from_sources(
    sources: &LandscapeSources,
    include_defense: bool,
) -> Result<LandscapeSnapshot> {
    let mut snap = LandscapeSnapshot::default();

    if let Some(dir) = &sources.sidecar_dir {
        merge_main_ledger(&mut snap, dir, RecordSource::Sidecar)?;
    }
    if let Some(dir) = &sources.working_dir {
        merge_main_ledger(&mut snap, dir, RecordSource::Working)?;
    }

    if include_defense {
        let mut defense_section = DefenseSection::default();
        if let Some(dir) = &sources.sidecar_dir {
            merge_defense(&mut defense_section, dir, RecordSource::Sidecar)?;
        }
        if let Some(dir) = &sources.working_dir {
            merge_defense(&mut defense_section, dir, RecordSource::Working)?;
        }
        snap.defense = Some(defense_section);
    }

    Ok(snap)
}

fn merge_main_ledger(
    snap: &mut LandscapeSnapshot,
    dir: &Path,
    source: RecordSource,
) -> Result<()> {
    for a in attractors::load(dir)? {
        let id = a.id.clone();
        upsert(&mut snap.attractors, |x| x.id == id, SnapshotAttractor {
            id: a.id,
            name: a.name,
            description: a.description,
            positive_state: a.positive_state,
            negative_state: a.negative_state,
            source,
        });
    }

    for s in stressors::load(dir)? {
        let id = s.id.clone();
        upsert(&mut snap.stressors, |x| x.id == id, SnapshotForce {
            id: s.id,
            shortname: s.shortname,
            description: s.description,
            naive_change: s.naive_change,
            outcomes: s.outcomes,
            attractor_id: s.attractor_id,
            kind: ForceKind::Stressor,
            source,
        });
    }

    for p in purposes::load(dir)? {
        let id = p.id.clone();
        upsert(&mut snap.purposes, |x| x.id == id, SnapshotForce {
            id: p.id,
            shortname: p.shortname,
            description: p.description,
            naive_change: p.naive_change,
            outcomes: p.outcomes,
            attractor_id: p.attractor_id,
            kind: ForceKind::Purpose,
            source,
        });
    }

    for c in components::load(dir)? {
        let name = c.name.clone();
        upsert(&mut snap.components, |x| x.name == name, SnapshotComponent {
            name: c.name,
            description: c.description,
            status: c.status,
            architecture_set: c.architecture_set,
            source,
        });
    }

    for r in residues::load(dir)? {
        let force_id = r.force_id.clone();
        let component_id = r.component_id.clone();
        let coupled = r.is_coupled();
        let whole_system = r.is_whole_system();
        upsert(
            &mut snap.residues,
            |x| x.force_id == force_id && x.component_id == component_id,
            SnapshotResidue {
                force_id: r.force_id,
                component_id: r.component_id,
                coupled,
                whole_system,
                notes: r.notes,
                source,
            },
        );
    }

    Ok(())
}

fn merge_defense(section: &mut DefenseSection, dir: &Path, source: RecordSource) -> Result<()> {
    for s in defense::meta_stressors::load(dir)? {
        let id = s.id.clone();
        upsert(
            &mut section.meta_stressors,
            |x| x.id == id,
            SnapshotMetaForce {
                id: s.id,
                shortname: s.shortname,
                description: s.description,
                naive_change: String::new(),
                outcomes: String::new(),
                attractor_id: String::new(),
                kind: ForceKind::Stressor,
                source,
            },
        );
    }

    for a in defense::meta_attractors::load(dir)? {
        let id = a.id.clone();
        upsert(
            &mut section.meta_attractors,
            |x| x.id == id,
            SnapshotAttractor {
                id: a.id,
                name: a.name,
                description: a.description,
                positive_state: a.positive_state,
                negative_state: a.negative_state,
                source,
            },
        );
    }

    for p in defense::meta_purposes::load(dir)? {
        let id = p.id.clone();
        upsert(
            &mut section.meta_purposes,
            |x| x.id == id,
            SnapshotMetaForce {
                id: p.id,
                shortname: p.shortname,
                description: p.description,
                naive_change: p.naive_change,
                outcomes: p.outcomes,
                attractor_id: p.attractor_id,
                kind: ForceKind::Purpose,
                source,
            },
        );
    }

    upsert_string_list(&mut section.personas, &defense::artifacts::list_personas(dir)?);
    upsert_string_list(
        &mut section.strategies,
        &defense::artifacts::list_strategies(dir)?,
    );
    upsert_string_list(&mut section.progress, &defense::artifacts::list_progress(dir)?);
    upsert_string_list(&mut section.pitches, &defense::artifacts::list_pitches(dir)?);

    Ok(())
}

fn upsert<T, F>(items: &mut Vec<T>, mut matches: F, item: T)
where
    F: FnMut(&T) -> bool,
{
    if let Some(existing) = items.iter_mut().find(|x| matches(x)) {
        *existing = item;
    } else {
        items.push(item);
    }
}

fn upsert_string_list(dest: &mut Vec<String>, incoming: &[String]) {
    for name in incoming {
        if !dest.iter().any(|x| x == name) {
            dest.push(name.clone());
        }
    }
}

/// Convenience wrapper for a single ledger directory.
pub fn load_landscape_snapshot_from_dir(
    dir: &Path,
    include_defense: bool,
) -> Result<LandscapeSnapshot> {
    load_landscape_snapshot_from_sources(
        &LandscapeSources {
            sidecar_dir: Some(dir.to_path_buf()),
            working_dir: None,
        },
        include_defense,
    )
}

/// Load the merged snapshot for a loaded config.
pub fn load_landscape_snapshot(cfg: &Config, include_defense: bool) -> Result<LandscapeSnapshot> {
    let sources = resolve_sources(cfg)?;
    load_landscape_snapshot_from_sources(&sources, include_defense)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::testing::{seed_defense_ledger, seed_main_ledger};
    use tempfile::tempdir;

    fn sources_for(sidecar: &Path, working: Option<&Path>) -> LandscapeSources {
        LandscapeSources {
            sidecar_dir: Some(sidecar.to_path_buf()),
            working_dir: working.map(Path::to_path_buf),
        }
    }

    /// Core read model: main ledger CSVs land in the snapshot with filterable fields.
    #[test]
    fn snapshot_loads_forces_attractors_components_and_residues() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);

        let snap = load_landscape_snapshot_from_dir(&ledger, false)
            .expect("snapshot must load from a seeded ledger dir");

        assert_eq!(snap.attractors.len(), 1, "expected A-01 in snapshot");
        assert_eq!(snap.attractors[0].id, "A-01");
        assert_eq!(snap.attractors[0].positive_state, "coherent NKP");

        assert_eq!(snap.stressors.len(), 1, "expected S-01 in snapshot");
        let s = &snap.stressors[0];
        assert_eq!(s.id, "S-01");
        assert_eq!(s.shortname, "queue-overload");
        assert_eq!(s.attractor_id, "A-01");
        assert_eq!(s.naive_change, "add retry");
        assert_eq!(s.kind, ForceKind::Stressor);

        assert_eq!(snap.purposes.len(), 1, "expected P-01 in snapshot");
        assert_eq!(snap.purposes[0].id, "P-01");
        assert_eq!(snap.purposes[0].kind, ForceKind::Purpose);

        let names: Vec<&str> = snap.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"auth"), "components: {names:?}");
        assert!(names.contains(&"db"), "components: {names:?}");
        assert_eq!(
            snap.components
                .iter()
                .find(|c| c.name == "auth")
                .map(|c| c.architecture_set.as_str()),
            Some("iter1"),
            "architecture_set must survive into the snapshot for filtering"
        );

        assert!(
            snap.residues
                .iter()
                .any(|r| r.force_id == "S-01" && r.component_id == "auth" && r.coupled),
            "S-01 × auth coupling must appear as a matrix cell, got {:?}",
            snap.residues
        );
    }

    /// `--defense` OFF: no defense section and no MS-/MA-/MP- leakage into JSON.
    #[test]
    fn snapshot_excludes_defense_by_default() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);

        let snap = load_landscape_snapshot_from_dir(&ledger, false)
            .expect("snapshot must load with defense present on disk but excluded");

        assert!(
            snap.defense.is_none(),
            "defense section must be absent without --defense"
        );

        let json = snap.to_json().expect("snapshot serializes to json");
        assert!(
            !json.contains("\"defense\""),
            "defense key must not be serialized at all, json={json}"
        );
        for needle in ["MS-01", "MA-01", "MP-01", "hostile-auditor", "alpha-strategy"] {
            assert!(
                !json.contains(needle),
                "defense record '{needle}' leaked into non-defense snapshot json={json}"
            );
        }
    }

    /// `--defense` ON: meta forces and defense artifacts are present.
    #[test]
    fn snapshot_includes_defense_with_flag() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);

        let snap = load_landscape_snapshot_from_dir(&ledger, true)
            .expect("snapshot must load with --defense");

        let defense = snap
            .defense
            .as_ref()
            .expect("defense section must be present with --defense");

        assert_eq!(defense.meta_stressors.len(), 1);
        assert_eq!(defense.meta_stressors[0].id, "MS-01");
        assert_eq!(defense.meta_stressors[0].kind, ForceKind::Stressor);

        assert_eq!(defense.meta_attractors.len(), 1);
        assert_eq!(defense.meta_attractors[0].id, "MA-01");

        assert_eq!(defense.meta_purposes.len(), 1);
        assert_eq!(defense.meta_purposes[0].id, "MP-01");
        assert_eq!(defense.meta_purposes[0].attractor_id, "MA-01");
        assert_eq!(defense.meta_purposes[0].kind, ForceKind::Purpose);

        assert!(
            defense.personas.iter().any(|p| p == "hostile-auditor"),
            "defense personas: {:?}",
            defense.personas
        );
        assert!(
            defense.strategies.iter().any(|s| s == "alpha-strategy"),
            "defense strategies: {:?}",
            defense.strategies
        );
        assert!(
            defense.progress.iter().any(|s| s == "week-1"),
            "defense progress: {:?}",
            defense.progress
        );
        assert!(
            defense.pitches.iter().any(|s| s == "exec-summary"),
            "defense pitches: {:?}",
            defense.pitches
        );

        let json = snap.to_json().expect("snapshot serializes to json");
        assert!(json.contains("\"defense\""), "json={json}");
        assert!(json.contains("MS-01"), "json={json}");
    }

    /// Main-ledger forces must never be classified into the defense section.
    #[test]
    fn snapshot_defense_section_never_absorbs_main_ledger_forces() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);

        let snap = load_landscape_snapshot_from_dir(&ledger, true)
            .expect("snapshot must load with --defense");
        let defense = snap.defense.as_ref().expect("defense section");

        assert!(
            defense.meta_stressors.iter().all(|m| m.id.starts_with("MS-")),
            "meta_stressors must only hold MS-*: {:?}",
            defense.meta_stressors
        );
        assert!(
            snap.stressors.iter().all(|s| s.id.starts_with("S-")),
            "main stressors must only hold S-*: {:?}",
            snap.stressors
        );
    }

    /// The snapshot reads capture-session metadata, not just the sidecar tip.
    #[test]
    fn snapshot_reads_capture_session_metadata_not_only_sidecar() {
        let dir = tempdir().unwrap();
        let sidecar = dir.path().join("sidecar-tip");
        let working = dir.path().join("capture-session");
        seed_main_ledger(&sidecar);
        seed_main_ledger(&working);
        // Capture-session-only stressor: staged by `branch edit`, not yet on the tip.
        std::fs::write(
            working.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-01,queue-overload,queue overload,add retry,requests drain,A-01\n\
S-02,session-only,uncommitted capture,inline zag,operator sees it,A-01\n",
        )
        .unwrap();

        let snap = load_landscape_snapshot_from_sources(
            &sources_for(&sidecar, Some(&working)),
            false,
        )
        .expect("snapshot must merge sidecar tip and capture-session metadata");

        let ids: Vec<&str> = snap.stressors.iter().map(|s| s.id.as_str()).collect();
        assert!(
            ids.contains(&"S-02"),
            "capture-session-only stressor must appear in the landscape, got {ids:?}"
        );
        assert_eq!(
            snap.stressors
                .iter()
                .find(|s| s.id == "S-02")
                .map(|s| s.source),
            Some(RecordSource::Working),
            "capture-session records must be tagged as working"
        );
        assert_eq!(
            snap.stressors.iter().filter(|s| s.id == "S-01").count(),
            1,
            "S-01 present in both sources must not be duplicated, got {ids:?}"
        );
    }

    /// Working metadata wins on id collision — the operator is editing it right now.
    #[test]
    fn snapshot_working_record_overrides_sidecar_record_on_same_id() {
        let dir = tempdir().unwrap();
        let sidecar = dir.path().join("sidecar-tip");
        let working = dir.path().join("capture-session");
        seed_main_ledger(&sidecar);
        seed_main_ledger(&working);
        std::fs::write(
            working.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-01,queue-overload,revised in capture session,add backpressure,requests drain,A-01\n",
        )
        .unwrap();

        let snap = load_landscape_snapshot_from_sources(
            &sources_for(&sidecar, Some(&working)),
            false,
        )
        .expect("snapshot must merge sidecar tip and capture-session metadata");

        let s = snap
            .stressors
            .iter()
            .find(|s| s.id == "S-01")
            .expect("S-01 must be present");
        assert_eq!(s.description, "revised in capture session");
        assert_eq!(s.source, RecordSource::Working);
    }

    /// Missing ledger dirs are an empty landscape, not an error.
    #[test]
    fn snapshot_from_empty_dir_is_empty_not_error() {
        let dir = tempdir().unwrap();
        let snap = load_landscape_snapshot_from_dir(dir.path(), false)
            .expect("empty ledger dir must yield an empty snapshot, not an error");
        assert!(snap.stressors.is_empty());
        assert!(snap.purposes.is_empty());
        assert!(snap.attractors.is_empty());
        assert!(snap.components.is_empty());
        assert!(snap.defense.is_none());
    }

    /// Inline (non-sidecar) config resolves to a single source, no duplicate working dir.
    #[test]
    fn resolve_sources_inline_config_has_no_separate_working_dir() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let cfg = Config::for_test_residual_dir(&ledger);

        let sources = resolve_sources(&cfg).expect("resolve_sources on inline config");
        assert_eq!(sources.sidecar_dir.as_deref(), Some(ledger.as_path()));
        assert_eq!(
            sources.working_dir, None,
            "inline config must not report the same dir twice"
        );
    }

    /// Config-driven entry point agrees with the dir-driven one for inline configs.
    #[test]
    fn load_landscape_snapshot_from_config_reads_inline_ledger() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let cfg = Config::for_test_residual_dir(&ledger);

        let snap = load_landscape_snapshot(&cfg, false)
            .expect("load_landscape_snapshot must read the inline ledger");
        assert_eq!(snap.stressors.len(), 1);
        assert_eq!(snap.stressors[0].id, "S-01");
        assert_eq!(snap.stressors[0].source, RecordSource::Sidecar);
    }
}
