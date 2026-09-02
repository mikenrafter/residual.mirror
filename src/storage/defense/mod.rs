//! Defense ledger — meta force namespaces isolated from main ledger.

pub mod meta_stressors;

use anyhow::Result;
use std::path::Path;

/// Initialize defense/ tree on effective (sidecar) residual directory.
pub fn init_tree(residual_dir: &Path) -> Result<()> {
    let _ = residual_dir;
    todo!("create defense/ tree with meta-stressors.csv header")
}

/// Verify MS-/MA-/MP- ids never appear in main ledger CSVs.
pub fn verify_meta_isolation(residual_dir: &Path) -> Result<()> {
    let _ = residual_dir;
    todo!("fail verify when meta force id bleeds into main stressors.csv")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::defense::meta_stressors::{append, list, MetaStressor};
    use tempfile::tempdir;

    #[test]
    fn meta_stressor_add_list_round_trip() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaStressor {
                id: "MS-01".into(),
                shortname: "meta-force-landscape".into(),
                description: "Defense-layer stressor".into(),
            },
        )
        .unwrap();

        let items = list(&residual).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "MS-01");
        assert_eq!(items[0].shortname, "meta-force-landscape");
    }

    #[test]
    fn meta_stressor_never_written_to_main_stressors_csv() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();
        std::fs::write(
            residual.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n",
        )
        .unwrap();
        init_tree(&residual).unwrap();

        append(
            &residual,
            MetaStressor {
                id: "MS-01".into(),
                shortname: "meta-only".into(),
                description: "Defense only".into(),
            },
        )
        .unwrap();

        let main = std::fs::read_to_string(residual.join("stressors.csv")).unwrap();
        assert!(
            !main.contains("MS-01") && !main.contains("MS-"),
            "MS-* must never appear in main stressors.csv, got: {main}"
        );
    }

    #[test]
    fn verify_fails_when_ms_id_in_main_stressors_csv() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();
        std::fs::write(
            residual.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
MS-01,contamination,bleed,none,,A-01\n",
        )
        .unwrap();

        let err = verify_meta_isolation(&residual).unwrap_err();
        assert!(
            err.to_string().contains("MS-") || err.to_string().contains("meta"),
            "verify must fail on MS- id in main stressors.csv"
        );
    }

    #[test]
    fn init_creates_defense_tree_on_effective_dir() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        std::fs::create_dir_all(&residual).unwrap();

        init_tree(&residual).unwrap();

        assert!(residual.join("defense").is_dir());
        assert!(residual.join("defense/meta-stressors.csv").exists());
    }
}
