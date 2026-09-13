//! Defense ledger structure — meta force schemas and validation.

use anyhow::Result;
use std::path::Path;

/// Returns true when force id uses meta namespace (MS-/MA-/MP-).
pub fn is_meta_force_id(id: &str) -> bool {
    id.starts_with("MS-") || id.starts_with("MA-") || id.starts_with("MP-")
}

fn scan_csv_id_column(path: &Path, label: &str, hits: &mut Vec<String>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path)?;
    let headers = rdr.headers()?.clone();
    let id_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("id"))
        .or_else(|| {
            if label == "residues.csv" {
                headers
                    .iter()
                    .position(|h| h.eq_ignore_ascii_case("force"))
            } else {
                None
            }
        });
    let Some(id_idx) = id_idx else {
        return Ok(());
    };
    for result in rdr.records() {
        let record = result?;
        if let Some(id) = record.get(id_idx) {
            let id = id.trim();
            if is_meta_force_id(id) {
                hits.push(format!("{label}: {id}"));
            }
        }
    }
    Ok(())
}

/// Scan main ledger CSVs for meta namespace contamination.
pub fn scan_main_ledger_contamination(residual_dir: &Path) -> Result<Vec<String>> {
    let mut hits = Vec::new();
    scan_csv_id_column(&residual_dir.join("stressors.csv"), "stressors.csv", &mut hits)?;
    scan_csv_id_column(&residual_dir.join("purposes.csv"), "purposes.csv", &mut hits)?;
    scan_csv_id_column(&residual_dir.join("attractors.csv"), "attractors.csv", &mut hits)?;
    scan_csv_id_column(&residual_dir.join("residues.csv"), "residues.csv", &mut hits)?;
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_meta_force_id_recognizes_ms_ma_mp_prefixes() {
        assert!(is_meta_force_id("MS-01"));
        assert!(is_meta_force_id("MA-01"));
        assert!(is_meta_force_id("MP-01"));
        assert!(!is_meta_force_id("S-01"));
        assert!(!is_meta_force_id("P-01"));
    }
}
