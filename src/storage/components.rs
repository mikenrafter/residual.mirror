//! Component registry mutations — append, idempotent add, residues.csv column extension.

use anyhow::Result;
use std::path::Path;

use crate::structure::definition::components::Component;

/// Append a component to components.csv and extend residues.csv header with its name column.
pub fn append(
    residual_dir: &Path,
    name: &str,
    description: &str,
    status: &str,
    architecture_set: &str,
) -> Result<()> {
    let _ = (residual_dir, name, description, status, architecture_set);
    todo!("append component registry row and extend residues.csv header column")
}

/// Idempotent append — no-op when component name already exists.
pub fn append_idempotent(
    residual_dir: &Path,
    name: &str,
    description: &str,
    status: &str,
    architecture_set: &str,
) -> Result<bool> {
    let _ = (residual_dir, name, description, status, architecture_set);
    todo!("idempotent component append")
}

pub fn load(residual_dir: &Path) -> Result<Vec<Component>> {
    crate::structure::definition::components::load(residual_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::integrity::sessions;
    use tempfile::tempdir;

    fn init_minimal(residual: &std::path::Path) {
        std::fs::create_dir_all(residual).unwrap();
        std::fs::write(
            residual.join("components.csv"),
            "name,description,status,architecture_set\n",
        )
        .unwrap();
        std::fs::write(residual.join("residues.csv"), "force\n").unwrap();
        std::fs::write(residual.join("config.toml"), "#\n").unwrap();
        sessions::write_snapshot(residual).unwrap();
    }

    #[test]
    fn append_component_extends_registry_and_residues_header() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        append(
            &residual,
            "storage-git-sidecar",
            "Git sidecar branch storage",
            "proposed",
            "iter4-storage",
        )
        .unwrap();

        let components = load(&residual).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "storage-git-sidecar");

        let header = std::fs::read_to_string(residual.join("residues.csv"))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string();
        assert!(
            header.contains("storage-git-sidecar"),
            "residues.csv header must gain component column, got: {header}"
        );
    }

    #[test]
    fn append_component_idempotent_on_same_name() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        append_idempotent(&residual, "cli", "Thin hub", "actual", "iter4-cli-hub").unwrap();
        let added = append_idempotent(&residual, "cli", "Thin hub", "actual", "iter4-cli-hub")
            .unwrap();
        assert!(!added, "second append with same name must be idempotent no-op");

        let components = load(&residual).unwrap();
        assert_eq!(
            components.len(),
            1,
            "idempotent append must not duplicate registry row"
        );
    }

    #[test]
    fn session_drift_requires_force_without_flag() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        let mut csv = std::fs::read_to_string(residual.join("components.csv")).unwrap();
        csv.push_str("smuggled,outside-session,actual,set\n");
        std::fs::write(residual.join("components.csv"), csv).unwrap();

        let err = append(&residual, "new-comp", "d", "proposed", "set").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("--force"),
            "drift without --force must mention --force, got: {msg}"
        );
    }
}
