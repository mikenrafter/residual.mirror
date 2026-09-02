//! Outsider / audience analysis — artifact routing and channel safety (S-52).

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudienceChannel {
    /// Hostile or untrusted channel — raw ledger paths blocked.
    RawUnsafe,
    /// Translated pitch — sanitized routing allowed.
    Translated,
}

/// Returns true when path points at defense ledger internals.
pub fn is_defense_artifact_path(path: &str) -> bool {
    path.contains("defense/") || path.contains("defense-personas/")
}

/// Returns true when path is a vetted translated pitch artifact.
pub fn is_translated_pitch_path(path: &str) -> bool {
    path.contains("defense/pitches/")
}

/// Route an artifact path for the given audience channel.
pub fn route_artifact(path: &str, channel: AudienceChannel) -> Result<()> {
    if !is_defense_artifact_path(path) {
        return Ok(());
    }

    match channel {
        AudienceChannel::RawUnsafe => {
            bail!(
                "reject: defense artifact path blocked on raw_unsafe channel: {path}"
            );
        }
        AudienceChannel::Translated if is_translated_pitch_path(path) => Ok(()),
        AudienceChannel::Translated => {
            bail!(
                "block: only defense/pitches/ paths allowed on translated channel: {path}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_unsafe_channel_rejects_walk_artifact_paths() {
        let paths = [
            "residual/defense/strategy/alpha.md",
            "residual/defense-personas/hostile-auditor.md",
            "residual/defense/meta-stressors.csv",
        ];
        for path in paths {
            let err = route_artifact(path, AudienceChannel::RawUnsafe).unwrap_err();
            assert!(
                err.to_string().contains("block")
                    || err.to_string().contains("unsafe")
                    || err.to_string().contains("reject"),
                "raw_unsafe must reject defense path {path}, got: {err}"
            );
        }
    }

    #[test]
    fn translated_pitch_passes_routing() {
        route_artifact("residual/defense/pitches/translated-summary.md", AudienceChannel::Translated)
            .expect("translated channel must allow sanitized pitch paths");
    }

    #[test]
    fn is_defense_artifact_path_detects_defense_tree() {
        assert!(is_defense_artifact_path("residual/defense/strategy/x.md"));
        assert!(is_defense_artifact_path("residual/defense-personas/voice.md"));
        assert!(!is_defense_artifact_path("residual/stressors.csv"));
    }
}
