use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stressor {
    pub id: String,
    #[serde(default)]
    pub shortname: String,
    pub description: String,
    pub attractor_id: String,
    pub naive_change: String,
    #[serde(rename = "outcomes", alias = "traits")]
    pub outcomes: String,
}

const HEADER: &str = "id,shortname,description,naive_change,outcomes,attractor_id";

pub fn load(residual_dir: &Path) -> Result<Vec<Stressor>> {
    let path = residual_dir.join("stressors.csv");
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)?;
    let mut result = Vec::new();
    for record in rdr.deserialize() {
        let s: Stressor = record?;
        result.push(s);
    }
    Ok(result)
}

pub fn append(residual_dir: &Path, stressor: Stressor) -> Result<()> {
    let mut all = load(residual_dir)?;
    if all.iter().any(|s| s.id == stressor.id) {
        anyhow::bail!("stressor id '{}' already exists", stressor.id);
    }
    all.push(stressor);
    write_all(residual_dir, &all)
}

pub fn remove_by_shortname(residual_dir: &Path, shortname: &str) -> Result<String> {
    let mut all = load(residual_dir)?;
    let Some(index) = all.iter().position(|s| s.shortname == shortname) else {
        anyhow::bail!("stressor '{}' not found", shortname);
    };
    let id = all.remove(index).id;
    write_all(residual_dir, &all)?;
    Ok(id)
}

pub fn write_all_pub(residual_dir: &Path, rows: &[Stressor]) -> Result<()> {
    write_all(residual_dir, rows)
}

fn write_all(residual_dir: &Path, rows: &[Stressor]) -> Result<()> {
    let mut buf = format!("{HEADER}\n");
    for s in rows {
        let mut row = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(&mut row);
            wtr.write_record(&[
                &s.id,
                &s.shortname,
                &s.description,
                &s.naive_change,
                &s.outcomes,
                &s.attractor_id,
            ])?;
            wtr.flush()?;
        }
        buf.push_str(std::str::from_utf8(&row)?);
    }
    std::fs::write(residual_dir.join("stressors.csv"), buf)?;
    Ok(())
}

/// Update fields on an existing stressor in place; unspecified fields are unchanged.
/// Also folds `add_component`/`remove_component` into residues.csv couplings for
/// this force id, mirroring `residual add/remove residue`. Errors on unknown id,
/// with no partial writes.
#[allow(clippy::too_many_arguments)]
pub fn update(
    residual_dir: &Path,
    id: &str,
    description: Option<String>,
    attractor_id: Option<String>,
    naive_change: Option<String>,
    shortname: Option<String>,
    outcomes: Option<String>,
    add_component: Vec<String>,
    remove_component: Vec<String>,
) -> Result<()> {
    let mut all = load(residual_dir)?;
    let Some(row) = all.iter_mut().find(|s| s.id == id) else {
        anyhow::bail!("stressor id '{}' not found", id);
    };
    if let Some(v) = description {
        row.description = v;
    }
    if let Some(v) = attractor_id {
        row.attractor_id = v;
    }
    if let Some(v) = naive_change {
        row.naive_change = v;
    }
    if let Some(v) = shortname {
        row.shortname = v;
    }
    if let Some(v) = outcomes {
        row.outcomes = v;
    }
    write_all(residual_dir, &all)?;

    for component_id in &add_component {
        let existing = crate::storage::residues::load(residual_dir)?;
        let residue_id = crate::storage::residues::next_id(&existing);
        crate::storage::residues::append(
            residual_dir,
            crate::structure::analysis::residues::Residue::coupling(
                residue_id,
                id.to_string(),
                component_id.clone(),
            ),
        )?;
    }
    for component_id in &remove_component {
        crate::storage::residues::remove_coupling(residual_dir, id, component_id)?;
    }
    Ok(())
}

pub fn next_id(stressors: &[Stressor]) -> String {
    let max = stressors
        .iter()
        .filter_map(|s| s.id.strip_prefix("S-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("S-{:02}", max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_stressor(id: &str) -> Stressor {
        Stressor {
            id: id.to_string(),
            shortname: String::new(),
            description: "desc".to_string(),
            attractor_id: "A-01".to_string(),
            naive_change: "change".to_string(),
            outcomes: "system handles auth".to_string(),
        }
    }

    #[test]
    fn next_id_empty() {
        assert_eq!(next_id(&[]), "S-01");
    }

    #[test]
    fn next_id_after_s03() {
        let stressors = vec![make_stressor("S-01"), make_stressor("S-03")];
        assert_eq!(next_id(&stressors), "S-04");
    }

    #[test]
    fn append_creates_file_with_header_and_row() {
        let dir = tempdir().unwrap();
        append(dir.path(), make_stressor("S-01")).unwrap();
        let content = std::fs::read_to_string(dir.path().join("stressors.csv")).unwrap();
        assert!(content.contains("id,"), "header missing");
        assert!(content.contains("S-01"), "row missing");
        assert!(!content.contains(",components,"));
    }

    #[test]
    fn stressor_with_shortname_roundtrips() {
        let dir = tempdir().unwrap();
        let s = Stressor {
            id: "S-01".to_string(),
            description: "test stressor".to_string(),
            attractor_id: "A-01".to_string(),
            naive_change: "fix it".to_string(),
            outcomes: "system handles stressor".to_string(),
            shortname: "cli-bypass".to_string(),
        };
        append(dir.path(), s).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded[0].shortname, "cli-bypass");
    }
}
