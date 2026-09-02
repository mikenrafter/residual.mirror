//! Residue persistence — matrix-shaped residues.csv (NKP coupling source).

use anyhow::{bail, Result};
use std::path::Path;

use crate::structure::analysis::residues::Residue;

pub fn load(residual_dir: &Path) -> Result<Vec<Residue>> {
    crate::storage::format::read_residues(residual_dir)
}

/// Upsert by (force_id, component_id); keeps matrix writes idempotent (A-04).
pub fn append(residual_dir: &Path, residue: Residue) -> Result<()> {
    let mut all = load(residual_dir)?;
    if let Some(existing) = all
        .iter_mut()
        .find(|r| r.force_id == residue.force_id && r.component_id == residue.component_id)
    {
        existing.status = "1".into();
        existing.notes.clear();
        if existing.id.is_empty() {
            existing.id = residue.id;
        }
    } else {
        all.push(residue);
    }
    crate::storage::format::write_residues(residual_dir, &all)
}

pub fn next_id(residues: &[Residue]) -> String {
    let max = residues
        .iter()
        .filter_map(|r| {
            r.id.strip_prefix("R-")
                .or_else(|| r.id.strip_prefix('R'))
                .and_then(|n| n.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("R-{:02}", max + 1)
}

pub fn force_exists(residual_dir: &Path, force_id: &str) -> Result<bool> {
    if crate::storage::stressors::load(residual_dir)?
        .iter()
        .any(|s| s.id == force_id)
    {
        return Ok(true);
    }
    if crate::storage::purposes::load(residual_dir)?
        .iter()
        .any(|p| p.id == force_id)
    {
        return Ok(true);
    }
    Ok(false)
}

pub fn append_whole_system(
    residual_dir: &Path,
    force_id: &str,
    notes: &str,
) -> Result<String> {
    if notes.trim().is_empty() {
        bail!("--whole-system requires --notes describing the hardware, process, organization, or policy zig");
    }
    if !force_exists(residual_dir, force_id)? {
        bail!("force id '{}' not found in stressors or purposes", force_id);
    }
    merge_force_notes(residual_dir, force_id, notes)?;
    let existing = load(residual_dir)?;
    let id = next_id(&existing);
    append(
        residual_dir,
        Residue::whole_system(id.clone(), force_id, notes),
    )?;
    Ok(id)
}

fn merge_force_notes(residual_dir: &Path, force_id: &str, notes: &str) -> Result<()> {
    let note = notes.trim();
    if note.is_empty() {
        return Ok(());
    }
    if let Ok(mut stressors) = crate::storage::stressors::load(residual_dir) {
        if let Some(s) = stressors.iter_mut().find(|s| s.id == force_id) {
            if !s.naive_change.contains(note) {
                if !s.naive_change.is_empty() {
                    s.naive_change.push(' ');
                }
                s.naive_change.push_str(note);
            }
            return crate::storage::stressors::write_all_pub(residual_dir, &stressors);
        }
    }
    if let Ok(mut purposes) = crate::storage::purposes::load(residual_dir) {
        if let Some(p) = purposes.iter_mut().find(|p| p.id == force_id) {
            if !p.naive_change.contains(note) {
                if !p.naive_change.is_empty() {
                    p.naive_change.push(' ');
                }
                p.naive_change.push_str(note);
            }
            return crate::storage::purposes::write_all_pub(residual_dir, &purposes);
        }
    }
    Ok(())
}

/// Clear a force×component matrix cell (remove residue coupling).
pub fn remove_coupling(
    residual_dir: &Path,
    force_id: &str,
    component_id: &str,
) -> Result<()> {
    let _ = (residual_dir, force_id, component_id);
    todo!("clear matrix cell for force_id × component_id")
}

/// Repoint coupling from one component column to another atomically.
pub fn move_coupling(
    residual_dir: &Path,
    force_id: &str,
    from_component: &str,
    to_component: &str,
) -> Result<()> {
    let _ = (residual_dir, force_id, from_component, to_component);
    todo!("repoint residue coupling via --move-to")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{format, stressors};
    use crate::structure::analysis::residues::Residue;
    use tempfile::tempdir;

    fn seed_matrix(residual: &Path) {
        std::fs::create_dir_all(residual).unwrap();
        std::fs::write(
            residual.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-01,alpha,desc,change,outcome,A-01\n",
        )
        .unwrap();
        std::fs::write(
            residual.join("components.csv"),
            "name,description,status,architecture_set\n\
auth,Auth module,actual,set\n\
db,Database,actual,set\n",
        )
        .unwrap();
        format::write_residues(
            residual,
            &[
                Residue::coupling("R-01", "S-01", "auth"),
                Residue::coupling("R-02", "S-01", "db"),
            ],
        )
        .unwrap();
    }

    #[test]
    fn remove_coupling_clears_matrix_cell() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        seed_matrix(&residual);

        remove_coupling(&residual, "S-01", "auth").unwrap();

        let residues = load(&residual).unwrap();
        assert!(
            !residues
                .iter()
                .any(|r| r.force_id == "S-01" && r.component_id == "auth" && r.is_coupled()),
            "auth coupling for S-01 must be cleared"
        );
        assert!(
            residues
                .iter()
                .any(|r| r.force_id == "S-01" && r.component_id == "db" && r.is_coupled()),
            "db coupling for S-01 must remain"
        );
    }

    #[test]
    fn move_coupling_repoints_atomically() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        seed_matrix(&residual);

        move_coupling(&residual, "S-01", "auth", "db").unwrap();

        let residues = load(&residual).unwrap();
        assert!(
            !residues
                .iter()
                .any(|r| r.force_id == "S-01" && r.component_id == "auth" && r.is_coupled()),
            "auth coupling must be cleared after move"
        );
        let db_coupled = residues
            .iter()
            .filter(|r| r.force_id == "S-01" && r.component_id == "db" && r.is_coupled())
            .count();
        assert_eq!(db_coupled, 1, "db must have exactly one coupling after move");
    }
}
