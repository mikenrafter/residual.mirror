//! Skills guru — embedded snippet registry keyed by topic.

use anyhow::Result;

pub const TOPIC_ATTRACTORS: &str = "attractors";
pub const TOPIC_STRESSORS: &str = "stressors";
pub const TOPIC_PURPOSES: &str = "purposes";
pub const TOPIC_PERSONAS: &str = "personas";
pub const TOPIC_WHOLE_SYSTEM_RESIDUE: &str = "whole-system-residue";
pub const TOPIC_WALK_REMINDER: &str = "walk-reminder";

const SNIPPET_ATTRACTORS: &str = include_str!("snippets/attractors.md");
const SNIPPET_STRESSORS: &str = include_str!("snippets/stressors.md");
const SNIPPET_PURPOSES: &str = include_str!("snippets/purposes.md");
const SNIPPET_PERSONAS: &str = include_str!("snippets/personas.md");
const SNIPPET_WHOLE_SYSTEM_RESIDUE: &str = include_str!("snippets/whole-system-residue.md");
const SNIPPET_WALK_REMINDER: &str = include_str!("snippets/walk-reminder.md");

/// Guru block for a topic, if any.
pub fn block_for_topic(topic: &str) -> Option<&'static str> {
    match topic {
        TOPIC_ATTRACTORS => Some(SNIPPET_ATTRACTORS),
        TOPIC_STRESSORS => Some(SNIPPET_STRESSORS),
        TOPIC_PURPOSES => Some(SNIPPET_PURPOSES),
        TOPIC_PERSONAS => Some(SNIPPET_PERSONAS),
        TOPIC_WHOLE_SYSTEM_RESIDUE => Some(SNIPPET_WHOLE_SYSTEM_RESIDUE),
        TOPIC_WALK_REMINDER => Some(SNIPPET_WALK_REMINDER),
        _ => None,
    }
}

/// Topics whose guru blocks attach to a skill's skill-data output.
pub fn topics_for_skill(skill_name: &str) -> &'static [&'static str] {
    match skill_name {
        "purpose-walk" => &[TOPIC_ATTRACTORS, TOPIC_PURPOSES],
        "stressor-walk" => &[
            TOPIC_ATTRACTORS,
            TOPIC_STRESSORS,
            TOPIC_PURPOSES,
            TOPIC_PERSONAS,
            TOPIC_WHOLE_SYSTEM_RESIDUE,
        ],
        "naive-draft" => &[TOPIC_PURPOSES],
        "integrate" => &[
            TOPIC_ATTRACTORS,
            TOPIC_STRESSORS,
            TOPIC_PURPOSES,
            TOPIC_WHOLE_SYSTEM_RESIDUE,
        ],
        "fmea" | "atam" => &[
            TOPIC_ATTRACTORS,
            TOPIC_STRESSORS,
            TOPIC_PURPOSES,
            TOPIC_PERSONAS,
            TOPIC_WHOLE_SYSTEM_RESIDUE,
        ],
        _ => &[
            TOPIC_ATTRACTORS,
            TOPIC_STRESSORS,
            TOPIC_PURPOSES,
            TOPIC_PERSONAS,
            TOPIC_WHOLE_SYSTEM_RESIDUE,
        ],
    }
}

/// Inject guru blocks into skill-data context for the given skill.
pub fn inject_for_skill(skill_name: &str, context: &str) -> Result<String> {
    let topics = topics_for_skill(skill_name);
    if topics.is_empty() {
        return Ok(context.to_string());
    }

    let mut out = context.to_string();
    out.push_str("\n## Guru\n\n");
    for (i, topic) in topics.iter().enumerate() {
        if let Some(block) = block_for_topic(topic) {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!("### {topic}\n\n"));
            out.push_str(block);
            if !block.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    Ok(out)
}

/// Token estimate for guru blocks attached to a skill.
pub fn token_estimate_for_skill(skill_name: &str) -> usize {
    topics_for_skill(skill_name)
        .iter()
        .filter_map(|topic| block_for_topic(topic))
        .map(|block| crate::skills::estimate_tokens(block))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::skills::context;
    use tempfile::tempdir;

    fn cfg_for(dir: &std::path::Path) -> Config {
        Config::for_test_residual_dir(dir)
    }

    #[test]
    fn stressor_walk_skill_data_contains_guru_whole_system_block() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        let out = context::build(&cfg, "stressor-walk").unwrap();
        assert!(
            out.contains("## Guru") || out.contains("whole-system-residue"),
            "stressor-walk skill-data must include guru block for whole-system-residue topic"
        );
        let block = block_for_topic(TOPIC_WHOLE_SYSTEM_RESIDUE);
        assert!(block.is_some(), "guru registry must define whole-system-residue topic");
    }

    #[test]
    fn naive_draft_skill_data_excludes_stressor_only_guru_blocks() {
        let dir = tempdir().unwrap();
        let cfg = cfg_for(dir.path());
        let out = context::build(&cfg, "naive-draft").unwrap();
        assert!(
            !out.contains("## Stressors") && !out.contains("stressor-walk guru"),
            "naive-draft must exclude stressor-only guru blocks"
        );
        if let Some(block) = block_for_topic("stressors") {
            assert!(
                !out.contains(block),
                "naive-draft context must not contain stressors guru snippet"
            );
        }
    }

    #[test]
    fn skill_list_includes_guru_token_estimate() {
        let estimate = token_estimate_for_skill("stressor-walk");
        assert!(
            estimate > 0,
            "skill list must report non-zero guru token estimate for stressor-walk"
        );
        let list = crate::skills::list_all_text();
        assert!(
            list.contains("guru") || list.contains("GURU") || list.contains("+guru"),
            "skill list output must include guru token estimate column or suffix"
        );
    }
}
