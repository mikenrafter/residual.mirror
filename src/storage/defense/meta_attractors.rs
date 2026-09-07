//! Meta-attractors — MA-* namespace in defense/meta-attractors.csv only.
//!
//! Stub for red TDD: green fills round-trip write/load.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaAttractor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub positive_state: String,
    pub negative_state: String,
}

const HEADER: &str = "id,name,description,positive_state,negative_state";
const CSV_NAME: &str = "defense/meta-attractors.csv";

fn csv_path(residual_dir: &Path) -> std::path::PathBuf {
    residual_dir.join(CSV_NAME)
}

pub fn load(residual_dir: &Path) -> Result<Vec<MetaAttractor>> {
    let path = csv_path(residual_dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)?;
    let mut result = Vec::new();
    for record in rdr.deserialize() {
        result.push(record?);
    }
    Ok(result)
}

pub fn append(residual_dir: &Path, item: MetaAttractor) -> Result<()> {
    let _ = (residual_dir, item, HEADER);
    bail!("TODO: meta_attractors::append — write MA-* to defense/meta-attractors.csv only")
}

pub fn list(residual_dir: &Path) -> Result<Vec<MetaAttractor>> {
    load(residual_dir)
}

pub fn next_id(items: &[MetaAttractor]) -> String {
    let max = items
        .iter()
        .filter_map(|s| s.id.strip_prefix("MA-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("MA-{:02}", max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::defense::init_tree;
    use tempfile::tempdir;

    #[test]
    fn meta_attractor_add_list_round_trip() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaAttractor {
                id: "MA-01".into(),
                name: "Practitioner Standing".into(),
                description: "Defense-layer attractor".into(),
                positive_state: "can discuss residuality safely".into(),
                negative_state: "method reads as heresy".into(),
            },
        )
        .expect("meta_attractors::append must succeed");

        let items = list(&residual).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "MA-01");
        assert_eq!(items[0].name, "Practitioner Standing");
    }

    #[test]
    fn meta_attractor_never_written_to_main_attractors_csv() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();
        std::fs::write(
            residual.join("attractors.csv"),
            "id,name,description,positive_state,negative_state\n",
        )
        .unwrap();
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaAttractor {
                id: "MA-01".into(),
                name: "meta-only".into(),
                description: "Defense only".into(),
                positive_state: "ok".into(),
                negative_state: "bad".into(),
            },
        )
        .expect("append must succeed without touching main attractors.csv");

        let main = std::fs::read_to_string(residual.join("attractors.csv")).unwrap();
        assert!(
            !main.contains("MA-01") && !main.contains("MA-"),
            "MA-* must never appear in main attractors.csv, got: {main}"
        );
        let meta = std::fs::read_to_string(residual.join("defense/meta-attractors.csv")).unwrap();
        assert!(
            meta.contains("MA-01"),
            "MA-* must land in defense/meta-attractors.csv, got: {meta}"
        );
    }

    #[test]
    fn next_id_uses_ma_prefix() {
        assert_eq!(next_id(&[]), "MA-01");
        assert_eq!(
            next_id(&[MetaAttractor {
                id: "MA-02".into(),
                name: "n".into(),
                description: "d".into(),
                positive_state: "p".into(),
                negative_state: "neg".into(),
            }]),
            "MA-03"
        );
    }
}
