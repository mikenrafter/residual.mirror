//! Meta-purposes — MP-* namespace in defense/meta-purposes.csv only.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaPurpose {
    pub id: String,
    #[serde(default)]
    pub shortname: String,
    pub description: String,
    pub naive_change: String,
    pub outcomes: String,
    pub attractor_id: String,
}

const HEADER: &str = "id,shortname,description,naive_change,outcomes,attractor_id";
const CSV_NAME: &str = "defense/meta-purposes.csv";

fn csv_path(residual_dir: &Path) -> std::path::PathBuf {
    residual_dir.join(CSV_NAME)
}

pub fn load(residual_dir: &Path) -> Result<Vec<MetaPurpose>> {
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

pub fn append(residual_dir: &Path, item: MetaPurpose) -> Result<()> {
    let mut all = load(residual_dir)?;
    if all.iter().any(|s| s.id == item.id) {
        anyhow::bail!("meta-purpose id '{}' already exists", item.id);
    }
    all.push(item);
    write_all(residual_dir, &all)
}

pub fn list(residual_dir: &Path) -> Result<Vec<MetaPurpose>> {
    load(residual_dir)
}

fn write_all(residual_dir: &Path, rows: &[MetaPurpose]) -> Result<()> {
    std::fs::create_dir_all(residual_dir.join("defense"))?;
    let mut buf = format!("{HEADER}\n");
    for p in rows {
        let mut row = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(&mut row);
            wtr.write_record(&[
                &p.id,
                &p.shortname,
                &p.description,
                &p.naive_change,
                &p.outcomes,
                &p.attractor_id,
            ])?;
            wtr.flush()?;
        }
        buf.push_str(std::str::from_utf8(&row)?);
    }
    std::fs::write(csv_path(residual_dir), buf)?;
    Ok(())
}

pub fn next_id(items: &[MetaPurpose]) -> String {
    let max = items
        .iter()
        .filter_map(|s| s.id.strip_prefix("MP-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("MP-{:02}", max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::defense::init_tree;
    use tempfile::tempdir;

    #[test]
    fn meta_purpose_add_list_round_trip() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaPurpose {
                id: "MP-01".into(),
                shortname: "defense-efficacy".into(),
                description: "Defense-layer purpose".into(),
                naive_change: "capture meta forces".into(),
                outcomes: "operator records defense purpose".into(),
                attractor_id: "MA-01".into(),
            },
        )
        .expect("meta_purposes::append must succeed");

        let items = list(&residual).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "MP-01");
        assert_eq!(items[0].shortname, "defense-efficacy");
    }

    #[test]
    fn meta_purpose_never_written_to_main_purposes_csv() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();
        std::fs::write(
            residual.join("purposes.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n",
        )
        .unwrap();
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaPurpose {
                id: "MP-01".into(),
                shortname: "meta-only".into(),
                description: "Defense only".into(),
                naive_change: "none".into(),
                outcomes: "".into(),
                attractor_id: "MA-01".into(),
            },
        )
        .expect("append must succeed without touching main purposes.csv");

        let main = std::fs::read_to_string(residual.join("purposes.csv")).unwrap();
        assert!(
            !main.contains("MP-01") && !main.contains("MP-"),
            "MP-* must never appear in main purposes.csv, got: {main}"
        );
        let meta = std::fs::read_to_string(residual.join("defense/meta-purposes.csv")).unwrap();
        assert!(
            meta.contains("MP-01"),
            "MP-* must land in defense/meta-purposes.csv, got: {meta}"
        );
    }

    #[test]
    fn next_id_uses_mp_prefix() {
        assert_eq!(next_id(&[]), "MP-01");
    }
}
