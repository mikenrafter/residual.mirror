use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Purpose {
    pub id: String,
    #[serde(default)]
    pub shortname: String,
    pub description: String,
    pub attractor_id: String,
    #[serde(alias = "feature")]
    pub naive_change: String,
    #[serde(rename = "outcomes", alias = "traits")]
    pub outcomes: String,
}

const HEADER: &str = "id,shortname,description,naive_change,outcomes,attractor_id";

pub fn load(residual_dir: &Path) -> Result<Vec<Purpose>> {
    let path = residual_dir.join("purposes.csv");
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)?;
    let mut result = Vec::new();
    for record in rdr.deserialize() {
        let p: Purpose = record?;
        result.push(p);
    }
    Ok(result)
}

pub fn append(residual_dir: &Path, purpose: Purpose) -> Result<()> {
    let mut all = load(residual_dir)?;
    if all.iter().any(|p| p.id == purpose.id) {
        anyhow::bail!("purpose id '{}' already exists", purpose.id);
    }
    all.push(purpose);
    write_all(residual_dir, &all)
}

pub fn write_all_pub(residual_dir: &Path, rows: &[Purpose]) -> Result<()> {
    write_all(residual_dir, rows)
}

fn write_all(residual_dir: &Path, rows: &[Purpose]) -> Result<()> {
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
    std::fs::write(residual_dir.join("purposes.csv"), buf)?;
    Ok(())
}

pub fn next_id(purposes: &[Purpose]) -> String {
    let max = purposes
        .iter()
        .filter_map(|p| p.id.strip_prefix("P-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("P-{:02}", max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_purpose(id: &str) -> Purpose {
        Purpose {
            id: id.to_string(),
            shortname: String::new(),
            description: "desc".to_string(),
            attractor_id: "A-01".to_string(),
            naive_change: "feat".to_string(),
            outcomes: "system enables login".to_string(),
        }
    }

    #[test]
    fn purpose_with_shortname_roundtrips() {
        let dir = tempdir().unwrap();
        let p = Purpose {
            id: "P-01".to_string(),
            description: "test purpose".to_string(),
            attractor_id: "A-01".to_string(),
            naive_change: "add purpose cli".to_string(),
            outcomes: "operator adds purposes".to_string(),
            shortname: "persona-subagent-depth".to_string(),
        };
        append(dir.path(), p).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded[0].shortname, "persona-subagent-depth");
    }
}
