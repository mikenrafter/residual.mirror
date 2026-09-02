//! Meta-stressors — MS-* namespace in defense/meta-stressors.csv only.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaStressor {
    pub id: String,
    #[serde(default)]
    pub shortname: String,
    pub description: String,
}

const HEADER: &str = "id,shortname,description";
const CSV_NAME: &str = "defense/meta-stressors.csv";

fn csv_path(residual_dir: &Path) -> std::path::PathBuf {
    residual_dir.join(CSV_NAME)
}

pub fn load(residual_dir: &Path) -> Result<Vec<MetaStressor>> {
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

pub fn append(residual_dir: &Path, item: MetaStressor) -> Result<()> {
    let mut all = load(residual_dir)?;
    if all.iter().any(|s| s.id == item.id) {
        anyhow::bail!("meta-stressor id '{}' already exists", item.id);
    }
    all.push(item);
    write_all(residual_dir, &all)
}

pub fn list(residual_dir: &Path) -> Result<Vec<MetaStressor>> {
    load(residual_dir)
}

fn write_all(residual_dir: &Path, rows: &[MetaStressor]) -> Result<()> {
    std::fs::create_dir_all(residual_dir.join("defense"))?;
    let mut buf = format!("{HEADER}\n");
    for s in rows {
        let mut row = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(&mut row);
            wtr.write_record(&[&s.id, &s.shortname, &s.description])?;
            wtr.flush()?;
        }
        buf.push_str(std::str::from_utf8(&row)?);
    }
    std::fs::write(csv_path(residual_dir), buf)?;
    Ok(())
}

pub fn next_id(items: &[MetaStressor]) -> String {
    let max = items
        .iter()
        .filter_map(|s| s.id.strip_prefix("MS-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("MS-{:02}", max + 1)
}
