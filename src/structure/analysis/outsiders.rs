//! Outsider / audience analysis — artifact routing and channel safety (S-52).

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudienceChannel {
    /// Hostile or untrusted channel — raw ledger paths blocked.
    RawUnsafe,
    /// Translated pitch — sanitized routing allowed.
    Translated,
}

/// Locked audience prior for defense-walk outsider simulation (S-52).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudiencePrior {
    pub name: &'static str,
    pub channel: AudienceChannel,
    /// When true, channel/safety priors must not be overridden mid-walk.
    pub locked: bool,
}

/// Built-in audience registry — locked priors + channel safety.
/// Stub for red TDD: green fills the locked prior set.
pub fn audience_registry() -> Vec<AudiencePrior> {
    Vec::new()
}

/// Resolve channel for a named audience; errors if unknown.
pub fn channel_for_audience(name: &str) -> Result<AudienceChannel> {
    let _ = name;
    bail!("TODO: channel_for_audience — look up locked audience registry")
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

/// Route using the locked audience registry (name → channel → path policy).
pub fn route_for_audience(audience: &str, path: &str) -> Result<()> {
    let channel = channel_for_audience(audience)?;
    route_artifact(path, channel)
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

    #[test]
    fn audience_registry_includes_locked_hostile_and_translated_priors() {
        let registry = audience_registry();
        assert!(
            !registry.is_empty(),
            "audience registry must include locked priors (S-52)"
        );
        assert!(
            registry.iter().any(|a| a.channel == AudienceChannel::RawUnsafe && a.locked),
            "must include at least one locked raw_unsafe (hostile) prior, got: {registry:?}"
        );
        assert!(
            registry
                .iter()
                .any(|a| a.channel == AudienceChannel::Translated && a.locked),
            "must include at least one locked translated prior, got: {registry:?}"
        );
    }

    #[test]
    fn route_for_audience_blocks_raw_ledger_for_hostile_prior() {
        let registry = audience_registry();
        let hostile = registry
            .iter()
            .find(|a| a.channel == AudienceChannel::RawUnsafe && a.locked)
            .expect("registry must define a locked hostile audience");
        let err = route_for_audience(hostile.name, "residual/defense/meta-stressors.csv")
            .expect_err("hostile audience must not receive raw defense ledger paths");
        assert!(
            err.to_string().contains("block")
                || err.to_string().contains("unsafe")
                || err.to_string().contains("reject"),
            "got: {err}"
        );
    }
}
