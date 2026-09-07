//! Defense markdown artifacts — personas, strategy, progress, pitches.
//!
//! Stub for red TDD: green fills writers + list helpers.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

pub fn persona_path(residual_dir: &Path, name: &str) -> PathBuf {
    residual_dir.join("defense-personas").join(format!("{name}.md"))
}

pub fn strategy_path(residual_dir: &Path, name: &str) -> PathBuf {
    residual_dir
        .join("defense/strategy")
        .join(format!("{name}.md"))
}

pub fn progress_path(residual_dir: &Path, name: &str) -> PathBuf {
    residual_dir
        .join("defense/progress")
        .join(format!("{name}.md"))
}

pub fn pitch_path(residual_dir: &Path, name: &str) -> PathBuf {
    residual_dir
        .join("defense/pitches")
        .join(format!("{name}.md"))
}

pub fn write_persona(residual_dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let _ = (residual_dir, name, body);
    bail!("TODO: write_persona → residual/defense-personas/<name>.md")
}

pub fn write_strategy(residual_dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let _ = (residual_dir, name, body);
    bail!("TODO: write_strategy → residual/defense/strategy/<name>.md")
}

pub fn write_progress(residual_dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let _ = (residual_dir, name, body);
    bail!("TODO: write_progress → residual/defense/progress/<name>.md")
}

pub fn write_pitch(residual_dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let _ = (residual_dir, name, body);
    bail!("TODO: write_pitch → residual/defense/pitches/<name>.md")
}

pub fn list_personas(residual_dir: &Path) -> Result<Vec<String>> {
    let _ = residual_dir;
    bail!("TODO: list_personas — surface defense-personas/*.md")
}

pub fn list_strategies(residual_dir: &Path) -> Result<Vec<String>> {
    let _ = residual_dir;
    bail!("TODO: list_strategies — surface defense/strategy/*.md")
}

pub fn list_progress(residual_dir: &Path) -> Result<Vec<String>> {
    let _ = residual_dir;
    bail!("TODO: list_progress — surface defense/progress/*.md")
}

pub fn list_pitches(residual_dir: &Path) -> Result<Vec<String>> {
    let _ = residual_dir;
    bail!("TODO: list_pitches — surface defense/pitches/*.md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::defense::init_tree;
    use tempfile::tempdir;

    #[test]
    fn write_defense_persona_lands_under_defense_personas() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        let path = write_persona(&residual, "hostile-auditor", "# Hostile auditor\n")
            .expect("write_persona must succeed");
        assert_eq!(path, persona_path(&residual, "hostile-auditor"));
        assert!(path.is_file(), "persona markdown must exist at {}", path.display());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("Hostile auditor"));
    }

    #[test]
    fn write_strategy_progress_pitch_land_under_defense_dirs() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        let strategy = write_strategy(&residual, "alpha", "# Strategy\n").unwrap();
        let progress = write_progress(&residual, "week-1", "# Progress\n").unwrap();
        let pitch = write_pitch(&residual, "exec-summary", "# Pitch\n").unwrap();

        assert_eq!(strategy, strategy_path(&residual, "alpha"));
        assert_eq!(progress, progress_path(&residual, "week-1"));
        assert_eq!(pitch, pitch_path(&residual, "exec-summary"));
        assert!(strategy.is_file());
        assert!(progress.is_file());
        assert!(pitch.is_file());
    }

    #[test]
    fn list_defense_artifacts_surfaces_written_names() {
        let dir = tempdir().unwrap();
        let residual = dir.path().join("residual");
        init_tree(&residual).unwrap();

        write_persona(&residual, "voice-a", "a").unwrap();
        write_strategy(&residual, "s1", "s").unwrap();
        write_progress(&residual, "p1", "p").unwrap();
        write_pitch(&residual, "pitch-1", "x").unwrap();

        let personas = list_personas(&residual).unwrap();
        let strategies = list_strategies(&residual).unwrap();
        let progress = list_progress(&residual).unwrap();
        let pitches = list_pitches(&residual).unwrap();

        assert!(personas.iter().any(|n| n.contains("voice-a")), "{personas:?}");
        assert!(strategies.iter().any(|n| n.contains("s1")), "{strategies:?}");
        assert!(progress.iter().any(|n| n.contains("p1")), "{progress:?}");
        assert!(pitches.iter().any(|n| n.contains("pitch-1")), "{pitches:?}");
    }
}
