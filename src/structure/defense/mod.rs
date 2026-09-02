//! Defense ledger structure — meta force schemas and validation.

use anyhow::Result;
use std::path::Path;

/// Returns true when force id uses meta namespace (MS-/MA-/MP-).
pub fn is_meta_force_id(id: &str) -> bool {
    id.starts_with("MS-") || id.starts_with("MA-") || id.starts_with("MP-")
}

/// Scan main ledger CSVs for meta namespace contamination.
pub fn scan_main_ledger_contamination(residual_dir: &Path) -> Result<Vec<String>> {
    let _ = residual_dir;
    todo!("scan main ledger for MS-/MA-/MP- contamination")
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
