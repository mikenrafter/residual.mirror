//! skill-hooks — adapter boundary for AI-agent lifecycle events (pre-compaction,
//! post-compaction, periodic conversation turns), distinct from the explicit
//! skill-session injection in `skills::guru`. Residue of P-33
//! (residual-ledger-continuity) and P-34 (contextual-guru-cadence).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::skills::guru;

/// Minimum turns before the same topic's suggestion may resurface (A-20: silence
/// when not relevant matters as much as firing when relevant — don't nag).
const MIN_TURNS_BETWEEN_SAME_TOPIC: u32 = 10;

const STATE_FILE_NAME: &str = "residual-hook-state.json";

/// Short prompt for the compaction process: keep decisions/dead-ends/corrections/
/// completed work, drop ceremony. Kept intentionally terse — this text becomes part
/// of a compaction prompt and its own token cost matters.
pub fn pre_compaction() -> String {
    "residual skill-hooks: preserve across compaction — decisions made, dead ends \
     explored, corrections applied, and residual-ledger additions already completed. \
     Drop narrative and ceremony; carry forward state, not story."
        .to_string()
}

/// Short reminder after compaction to consult skills-guru before picking next steps.
pub fn post_compaction() -> String {
    format!(
        "residual skill-hooks: before deciding next steps, consult skills-guru for the \
         relevant topic — {}, {}, {}, {}, {}, or {}.",
        guru::TOPIC_ATTRACTORS,
        guru::TOPIC_STRESSORS,
        guru::TOPIC_PURPOSES,
        guru::TOPIC_PERSONAS,
        guru::TOPIC_WHOLE_SYSTEM_RESIDUE,
        guru::TOPIC_WALK_REMINDER,
    )
}

/// Classify a discussion snippet against `skills::guru` topics and, subject to
/// per-topic cadence and dedup, emit a suggestion pointing at the matching guru
/// block. Silent (`Ok(None)`) whenever nothing matches or cadence hasn't elapsed —
/// per A-20, staying absent is as important as firing.
pub fn periodic_turn(turn: u32, text: &str) -> Result<Option<String>> {
    let path = state_file_path()?;
    periodic_turn_at(&path, turn, text)
}

fn periodic_turn_at(state_path: &Path, turn: u32, text: &str) -> Result<Option<String>> {
    let topic = match classify(text) {
        Some(topic) => topic,
        None => return Ok(None),
    };

    let mut state = load_state(state_path)?;
    if let Some(&last_turn) = state.last_fired_turn.get(topic) {
        if turn.saturating_sub(last_turn) < MIN_TURNS_BETWEEN_SAME_TOPIC {
            return Ok(None);
        }
    }

    let suggestion = build_suggestion(topic, turn);
    if state.last_suggestion == suggestion {
        return Ok(None);
    }

    state.last_fired_turn.insert(topic.to_string(), turn);
    state.last_suggestion = suggestion.clone();
    save_state(state_path, &state)?;
    Ok(Some(suggestion))
}

/// Intentionally simple: case-insensitive substring/keyword matching against a
/// short hand-picked list per topic — no model calls, no new dependencies. This is
/// a heuristic router toward skills-guru topics, not a classifier that needs to be
/// precise; false negatives (silence) are the safe failure mode per A-20.
fn classify(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    const TOPICS: &[&str] = &[
        guru::TOPIC_ATTRACTORS,
        guru::TOPIC_STRESSORS,
        guru::TOPIC_PURPOSES,
        guru::TOPIC_PERSONAS,
        guru::TOPIC_WHOLE_SYSTEM_RESIDUE,
        guru::TOPIC_WALK_REMINDER,
    ];
    for topic in TOPICS {
        if lower.contains(topic) || topic_keywords(topic).iter().any(|kw| lower.contains(kw)) {
            return Some(topic);
        }
    }
    None
}

fn topic_keywords(topic: &str) -> &'static [&'static str] {
    match topic {
        guru::TOPIC_ATTRACTORS => &["attractor", "positive state", "negative state"],
        guru::TOPIC_STRESSORS => &["stressor", "naive change", "naive draft"],
        guru::TOPIC_PURPOSES => &["purpose", "behavioral contract"],
        guru::TOPIC_PERSONAS => &["persona", "simulation voice"],
        guru::TOPIC_WHOLE_SYSTEM_RESIDUE => &["whole-system", "whole system"],
        guru::TOPIC_WALK_REMINDER => &["purpose-walk", "stressor-walk", "walk cadence"],
        _ => &[],
    }
}

fn build_suggestion(topic: &str, turn: u32) -> String {
    match guru::block_for_topic(topic) {
        Some(block) => format!(
            "[skill-hooks turn {turn}] discussion touches '{topic}' — skills-guru: {}",
            block.trim()
        ),
        None => format!(
            "[skill-hooks turn {turn}] discussion touches '{topic}' — see `residual skill data` for guidance."
        ),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HookState {
    #[serde(default)]
    last_fired_turn: BTreeMap<String, u32>,
    #[serde(default)]
    last_suggestion: String,
}

fn load_state(path: &Path) -> Result<HookState> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
        Err(_) => Ok(HookState::default()),
    }
}

fn save_state(path: &Path, state: &HookState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state).context("serialize hook state")?;
    std::fs::write(path, raw).with_context(|| format!("write {}", path.display()))
}

/// State lives under `.git/`, never under `residual/` (git-sidecar-tracked ledger
/// data) — this is per-checkout runtime scratch, analogous in spirit to
/// `git_sidecar::scratch_path`, and must never be committed.
fn state_file_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("get current dir")?;
    Ok(cwd.join(".git").join(STATE_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pre_compaction_preserves_decisions_dead_ends_corrections_completed_work() {
        let text = pre_compaction().to_lowercase();
        for expected in ["decision", "dead end", "correction", "completed"] {
            assert!(text.contains(expected), "expected '{expected}' in: {text}");
        }
        assert!(
            text.contains("ceremony") || text.contains("narrative"),
            "expected pre_compaction to deprioritize ceremony/narrative, got: {text}"
        );
    }

    #[test]
    fn pre_compaction_is_genuinely_compact() {
        let text = pre_compaction();
        assert!(
            text.len() < 500,
            "pre_compaction prompt should stay compact, got {} chars",
            text.len()
        );
    }

    #[test]
    fn post_compaction_names_every_guru_topic() {
        let text = post_compaction();
        for topic in [
            guru::TOPIC_ATTRACTORS,
            guru::TOPIC_STRESSORS,
            guru::TOPIC_PURPOSES,
            guru::TOPIC_PERSONAS,
            guru::TOPIC_WHOLE_SYSTEM_RESIDUE,
            guru::TOPIC_WALK_REMINDER,
        ] {
            assert!(text.contains(topic), "expected '{topic}' in: {text}");
        }
        assert!(
            text.to_lowercase().contains("skills-guru"),
            "expected explicit skills-guru pointer, got: {text}"
        );
    }

    #[test]
    fn periodic_turn_silent_when_nothing_matches() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let result = periodic_turn_at(&state_path, 1, "just chatting about lunch plans").unwrap();
        assert_eq!(result, None, "no guru topic keywords present, must stay silent");
        assert!(
            !state_path.exists(),
            "silent no-op must not create a state file"
        );
    }

    #[test]
    fn periodic_turn_fires_on_first_match_and_records_it() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let result =
            periodic_turn_at(&state_path, 5, "we need to talk about the attractor states").unwrap();
        assert!(result.is_some(), "expected a suggestion on first match");
        let suggestion = result.unwrap();
        assert!(suggestion.contains(guru::TOPIC_ATTRACTORS));
        assert!(state_path.is_file(), "firing must persist cadence state");
    }

    #[test]
    fn periodic_turn_stays_silent_within_cadence_window() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let first = periodic_turn_at(&state_path, 1, "discussing the stressor list").unwrap();
        assert!(first.is_some());

        let second = periodic_turn_at(&state_path, 5, "more on the stressor list").unwrap();
        assert_eq!(
            second, None,
            "same topic within MIN_TURNS_BETWEEN_SAME_TOPIC must stay silent"
        );
    }

    #[test]
    fn periodic_turn_fires_again_after_cadence_window_elapses() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let first = periodic_turn_at(&state_path, 1, "discussing the stressor list").unwrap();
        assert!(first.is_some());

        let later = periodic_turn_at(
            &state_path,
            1 + MIN_TURNS_BETWEEN_SAME_TOPIC,
            "discussing the stressor list",
        )
        .unwrap();
        assert!(
            later.is_some(),
            "cadence window elapsed, topic still relevant — must fire again"
        );
        assert_ne!(
            first.unwrap(),
            later.unwrap(),
            "suggestions at different turns must not be byte-identical"
        );
    }

    #[test]
    fn periodic_turn_dedup_suppresses_exact_repeat_suggestion() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");

        // Seed state as if the same suggestion the next call would produce already
        // fired long enough ago that cadence alone would allow it again.
        let would_fire_turn = 50;
        let suggestion = build_suggestion(guru::TOPIC_PERSONAS, would_fire_turn);
        let mut seeded = HookState::default();
        seeded.last_fired_turn.insert(guru::TOPIC_PERSONAS.to_string(), 1);
        seeded.last_suggestion = suggestion;
        save_state(&state_path, &seeded).unwrap();

        let result = periodic_turn_at(&state_path, would_fire_turn, "let's discuss personas").unwrap();
        assert_eq!(
            result, None,
            "dedup must suppress an exact repeat of the last emitted suggestion"
        );
    }

    #[test]
    fn periodic_turn_topics_are_independent_under_cadence() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let stressor_hit = periodic_turn_at(&state_path, 1, "discussing the stressor list").unwrap();
        assert!(stressor_hit.is_some());

        let purpose_hit =
            periodic_turn_at(&state_path, 2, "what is the purpose behavioral contract here").unwrap();
        assert!(
            purpose_hit.is_some(),
            "a different topic must not be gated by another topic's cadence"
        );
    }

    #[test]
    fn classify_is_case_insensitive() {
        assert_eq!(classify("ATTRACTOR review time"), Some(guru::TOPIC_ATTRACTORS));
        assert_eq!(classify("nothing relevant here"), None);
    }

    #[test]
    fn load_state_recovers_from_corrupt_file_instead_of_failing() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        std::fs::write(&state_path, "not json at all").unwrap();
        let state = load_state(&state_path).unwrap();
        assert!(state.last_fired_turn.is_empty());
    }
}
