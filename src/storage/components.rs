//! Component registry mutations — append, idempotent add, residues.csv column extension.

use anyhow::Result;
use std::path::Path;

use crate::structure::definition::components::Component;

const HEADER: &str = "name,description,status,architecture_set";

fn write_all(residual_dir: &Path, rows: &[Component]) -> Result<()> {
    let mut buf = format!("{HEADER}\n");
    for c in rows {
        let mut row = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(&mut row);
            wtr.write_record(&[
                &c.name,
                &c.description,
                &c.status,
                &c.architecture_set,
            ])?;
            wtr.flush()?;
        }
        buf.push_str(std::str::from_utf8(&row)?);
    }
    std::fs::write(residual_dir.join("components.csv"), buf)?;
    Ok(())
}

fn extend_residues_header(residual_dir: &Path, component_name: &str) -> Result<()> {
    let path = residual_dir.join("residues.csv");
    let text = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        "force\n".to_string()
    };
    let ends_with_newline = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if lines.is_empty() {
        lines.push("force".to_string());
    }
    let header_parts: Vec<&str> = lines[0].split(',').map(str::trim).collect();
    if header_parts.iter().any(|c| *c == component_name) {
        return Ok(());
    }
    lines[0] = format!("{},{}", lines[0].trim_end_matches(','), component_name);
    for line in lines.iter_mut().skip(1) {
        if !line.trim().is_empty() {
            line.push(',');
        }
    }
    let mut out = lines.join("\n");
    if ends_with_newline || lines.len() == 1 {
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// Idempotent append — no-op when component name already exists. No session guard (caller holds session).
pub(crate) fn append_idempotent_inner(
    residual_dir: &Path,
    name: &str,
    description: &str,
    status: &str,
    architecture_set: &str,
) -> Result<bool> {
    let mut components = load(residual_dir)?;
    if components.iter().any(|c| c.name == name) {
        return Ok(false);
    }
    components.push(Component {
        name: name.to_string(),
        description: description.to_string(),
        status: status.to_string(),
        architecture_set: architecture_set.to_string(),
    });
    write_all(residual_dir, &components)?;
    extend_residues_header(residual_dir, name)?;
    Ok(true)
}

/// Append a component to components.csv and extend residues.csv header with its name column.
pub fn append(
    residual_dir: &Path,
    name: &str,
    description: &str,
    status: &str,
    architecture_set: &str,
) -> Result<()> {
    let session = crate::storage::integrity::sessions::begin_mutation(residual_dir, false)?;
    append_idempotent_inner(residual_dir, name, description, status, architecture_set)?;
    session.commit()?;
    Ok(())
}

/// Idempotent append — no-op when component name already exists.
pub fn append_idempotent(
    residual_dir: &Path,
    name: &str,
    description: &str,
    status: &str,
    architecture_set: &str,
) -> Result<bool> {
    let session = crate::storage::integrity::sessions::begin_mutation(residual_dir, false)?;
    let added = append_idempotent_inner(residual_dir, name, description, status, architecture_set)?;
    session.commit()?;
    Ok(added)
}

/// Update status for an existing component (e.g. proposed → actual). Errors if name unknown.
/// No session guard — caller holds the mutation session.
pub(crate) fn set_status_inner(residual_dir: &Path, name: &str, status: &str) -> Result<()> {
    let mut components = load(residual_dir)?;
    let Some(row) = components.iter_mut().find(|c| c.name == name) else {
        anyhow::bail!("unknown component: {name} not found");
    };
    row.status = status.to_string();
    write_all(residual_dir, &components)?;
    Ok(())
}

/// Update status for an existing component (e.g. proposed → actual). Errors if name unknown.
/// Session/change-detection guarded like other mutators.
pub fn set_status(residual_dir: &Path, name: &str, status: &str) -> Result<()> {
    let session = crate::storage::integrity::sessions::begin_mutation(residual_dir, false)?;
    set_status_inner(residual_dir, name, status)?;
    session.commit()?;
    Ok(())
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

    #[test]
    fn set_status_promotes_proposed_to_actual() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        append(
            &residual,
            "skills-defense-walk",
            "Defense walk skill",
            "proposed",
            "iter4-defense",
        )
        .unwrap();

        set_status(&residual, "skills-defense-walk", "actual")
            .expect("set_status must update existing component");

        let components = load(&residual).unwrap();
        let row = components
            .iter()
            .find(|c| c.name == "skills-defense-walk")
            .expect("component must still exist");
        assert_eq!(row.status, "actual");
    }

    #[test]
    fn set_status_unknown_name_errors() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        let err = set_status(&residual, "does-not-exist", "actual").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("does-not-exist")
                || msg.contains("unknown")
                || msg.contains("not found")
                || msg.contains("no such"),
            "unknown name must error clearly, got: {msg}"
        );
        // TODO stub message is not the final contract — green must replace with real not-found.
        assert!(
            !msg.contains("TODO: components::set_status"),
            "set_status must be implemented (not stub TODO), got: {msg}"
        );
    }

    #[test]
    fn set_status_respects_session_drift() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_minimal(&residual);

        append(
            &residual,
            "skills-guru",
            "Guru snippets",
            "proposed",
            "iter4-guru",
        )
        .unwrap();

        let mut csv = std::fs::read_to_string(residual.join("components.csv")).unwrap();
        csv.push_str("smuggled,outside-session,actual,set\n");
        std::fs::write(residual.join("components.csv"), csv).unwrap();

        let err = set_status(&residual, "skills-guru", "actual").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("--force"),
            "set_status drift without --force must mention --force, got: {msg}"
        );
    }
}
