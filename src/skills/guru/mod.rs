//! Skills guru — embedded snippet registry keyed by topic.

use anyhow::Result;

pub const TOPIC_WHOLE_SYSTEM_RESIDUE: &str = "whole-system-residue";
pub const TOPIC_WALK_REMINDER: &str = "walk-reminder";

/// Guru block for a topic, if any.
pub fn block_for_topic(topic: &str) -> Option<&'static str> {
    let _ = topic;
    todo!("return guru snippet for topic")
}

/// Inject guru blocks into skill-data context for the given skill.
pub fn inject_for_skill(skill_name: &str, context: &str) -> Result<String> {
    let _ = (skill_name, context);
    todo!("inject guru blocks into skill-data output")
}

/// Token estimate for guru blocks attached to a skill.
pub fn token_estimate_for_skill(skill_name: &str) -> usize {
    let _ = skill_name;
    todo!("estimate guru token overhead for skill list")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::skills::context;
    use tempfile::tempdir;

    fn cfg_for(dir: &std::path::Path) -> Config {
        Config {
            validation: crate::config::ValidationConfig { strict: true },
            skills: crate::config::SkillsConfig { token_warn: 1000 },
            residual_dir: dir.to_path_buf(),
        }
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
