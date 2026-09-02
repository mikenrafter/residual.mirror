//! Components schema — registry, status, architecture_set.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub description: String,
    pub status: String,
    pub architecture_set: String,
}

pub fn load(residual_dir: &Path) -> Result<Vec<Component>> {
    let path = residual_dir.join("components.csv");
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

pub fn filter_architecture_set<'a>(
    components: &'a [Component],
    set: &str,
) -> Vec<&'a Component> {
    components
        .iter()
        .filter(|c| c.architecture_set == set)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_parses_name_description_status_architecture_set_columns() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("components.csv"),
            "name,description,status,architecture_set\ncli,Thin hub,actual,iter4-cli-hub\n",
        )
        .unwrap();
        let components = load(dir.path()).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "cli");
        assert_eq!(components[0].description, "Thin hub");
        assert_eq!(components[0].status, "actual");
        assert_eq!(components[0].architecture_set, "iter4-cli-hub");
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = TempDir::new().unwrap();
        assert!(load(dir.path()).unwrap().is_empty());
    }
}
