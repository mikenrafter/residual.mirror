//! Command generator — turns UI-staged adds into copy-pasteable `residual add …`
//! shell commands. The landscape UI never mutates the ledger; the operator runs
//! the emitted commands themselves.

use serde::{Deserialize, Serialize};

/// One add staged in the landscape UI. Variants and field names mirror
/// [`crate::cli::AddTarget`] so the emitted flags stay in lockstep with clap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "kebab-case")]
pub enum StagedAdd {
    Stressor {
        description: String,
        attractor_id: String,
        naive_change: String,
        #[serde(default)]
        shortname: String,
        #[serde(default)]
        outcomes: String,
    },
    Purpose {
        description: String,
        attractor_id: String,
        naive_change: String,
        #[serde(default)]
        shortname: String,
        #[serde(default)]
        outcomes: String,
    },
    Attractor {
        name: String,
        description: String,
        positive_state: String,
        negative_state: String,
    },
    Component {
        name: String,
        description: String,
        status: String,
        architecture_set: String,
    },
    MetaStressor {
        description: String,
        #[serde(default)]
        shortname: String,
    },
    MetaAttractor {
        name: String,
        description: String,
        positive_state: String,
        negative_state: String,
    },
    MetaPurpose {
        description: String,
        attractor_id: String,
        naive_change: String,
        #[serde(default)]
        shortname: String,
        #[serde(default)]
        outcomes: String,
    },
    DefensePersona {
        name: String,
        body: String,
    },
    DefenseStrategy {
        name: String,
        body: String,
    },
    DefenseProgress {
        name: String,
        body: String,
    },
    DefensePitch {
        name: String,
        body: String,
    },
}

impl StagedAdd {
    /// `residual add <subcommand>` name as clap spells it (kebab-case).
    pub fn subcommand(&self) -> &'static str {
        match self {
            StagedAdd::Stressor { .. } => "stressor",
            StagedAdd::Purpose { .. } => "purpose",
            StagedAdd::Attractor { .. } => "attractor",
            StagedAdd::Component { .. } => "component",
            StagedAdd::MetaStressor { .. } => "meta-stressor",
            StagedAdd::MetaAttractor { .. } => "meta-attractor",
            StagedAdd::MetaPurpose { .. } => "meta-purpose",
            StagedAdd::DefensePersona { .. } => "defense-persona",
            StagedAdd::DefenseStrategy { .. } => "defense-strategy",
            StagedAdd::DefenseProgress { .. } => "defense-progress",
            StagedAdd::DefensePitch { .. } => "defense-pitch",
        }
    }

    /// True for variants that only belong in a `--defense` landscape.
    pub fn is_defense(&self) -> bool {
        matches!(
            self,
            StagedAdd::MetaStressor { .. }
                | StagedAdd::MetaAttractor { .. }
                | StagedAdd::MetaPurpose { .. }
                | StagedAdd::DefensePersona { .. }
                | StagedAdd::DefenseStrategy { .. }
                | StagedAdd::DefenseProgress { .. }
                | StagedAdd::DefensePitch { .. }
        )
    }
}

/// Quote a value for safe copy-paste into a shell.
///
/// Single-line values use POSIX single quotes (`'\''` for embedded apostrophes).
/// Values containing newlines use ANSI-C `$'…'` quoting so the whole argument
/// stays on one command line.
pub fn shell_quote(value: &str) -> String {
    if value.contains('\n') {
        let mut escaped = String::with_capacity(value.len() + 8);
        for ch in value.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '\'' => escaped.push_str("\\'"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                c => escaped.push(c),
            }
        }
        format!("$'{escaped}'")
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn push_flag(parts: &mut Vec<String>, flag: &str, value: &str) {
    parts.push(format!("{flag} {}", shell_quote(value)));
}

fn push_optional_flag(parts: &mut Vec<String>, flag: &str, value: &str) {
    if !value.is_empty() {
        push_flag(parts, flag, value);
    }
}

/// Format one staged add as a single-line `residual add …` command.
pub fn format_add_command(staged: &StagedAdd) -> String {
    let mut parts = vec![format!("residual add {}", staged.subcommand())];
    match staged {
        StagedAdd::Stressor {
            description,
            attractor_id,
            naive_change,
            shortname,
            outcomes,
        } => {
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--attractor-id", attractor_id);
            push_flag(&mut parts, "--naive-change", naive_change);
            push_optional_flag(&mut parts, "--shortname", shortname);
            push_optional_flag(&mut parts, "--outcomes", outcomes);
        }
        StagedAdd::Purpose {
            description,
            attractor_id,
            naive_change,
            shortname,
            outcomes,
        } => {
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--attractor-id", attractor_id);
            push_flag(&mut parts, "--naive-change", naive_change);
            push_optional_flag(&mut parts, "--shortname", shortname);
            push_optional_flag(&mut parts, "--outcomes", outcomes);
        }
        StagedAdd::Attractor {
            name,
            description,
            positive_state,
            negative_state,
        } => {
            push_flag(&mut parts, "--name", name);
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--positive-state", positive_state);
            push_flag(&mut parts, "--negative-state", negative_state);
        }
        StagedAdd::Component {
            name,
            description,
            status,
            architecture_set,
        } => {
            push_flag(&mut parts, "--name", name);
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--status", status);
            push_flag(&mut parts, "--architecture-set", architecture_set);
        }
        StagedAdd::MetaStressor {
            description,
            shortname,
        } => {
            push_flag(&mut parts, "--description", description);
            push_optional_flag(&mut parts, "--shortname", shortname);
        }
        StagedAdd::MetaAttractor {
            name,
            description,
            positive_state,
            negative_state,
        } => {
            push_flag(&mut parts, "--name", name);
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--positive-state", positive_state);
            push_flag(&mut parts, "--negative-state", negative_state);
        }
        StagedAdd::MetaPurpose {
            description,
            attractor_id,
            naive_change,
            shortname,
            outcomes,
        } => {
            push_flag(&mut parts, "--description", description);
            push_flag(&mut parts, "--attractor-id", attractor_id);
            push_flag(&mut parts, "--naive-change", naive_change);
            push_optional_flag(&mut parts, "--shortname", shortname);
            push_optional_flag(&mut parts, "--outcomes", outcomes);
        }
        StagedAdd::DefensePersona { name, body }
        | StagedAdd::DefenseStrategy { name, body }
        | StagedAdd::DefenseProgress { name, body }
        | StagedAdd::DefensePitch { name, body } => {
            push_flag(&mut parts, "--name", name);
            push_flag(&mut parts, "--body", body);
        }
    }
    parts.join(" ")
}

/// Format staged adds as a newline-separated command list (no trailing newline).
pub fn format_add_commands(staged: &[StagedAdd]) -> String {
    staged
        .iter()
        .map(format_add_command)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stressor() -> StagedAdd {
        StagedAdd::Stressor {
            description: "queue overload".into(),
            attractor_id: "A-01".into(),
            naive_change: "add retry".into(),
            shortname: "queue-overload".into(),
            outcomes: "requests drain".into(),
        }
    }

    /// The headline contract: emitted stressor command matches real clap flags.
    #[test]
    fn command_generator_emits_valid_add_stressor_shape() {
        let cmd = format_add_command(&stressor());

        assert!(
            cmd.starts_with("residual add stressor "),
            "command must start with `residual add stressor `, got: {cmd}"
        );
        for flag in [
            "--description 'queue overload'",
            "--attractor-id 'A-01'",
            "--naive-change 'add retry'",
            "--shortname 'queue-overload'",
            "--outcomes 'requests drain'",
        ] {
            assert!(cmd.contains(flag), "missing {flag} in: {cmd}");
        }
        assert!(
            !cmd.contains('\n'),
            "one staged add must be one line, got: {cmd}"
        );
        assert!(
            !cmd.contains("--traits"),
            "emit the canonical --outcomes flag, not the --traits alias: {cmd}"
        );
    }

    /// Optional flags are dropped when blank so the command stays copy-pasteable.
    #[test]
    fn command_generator_omits_blank_optional_flags() {
        let cmd = format_add_command(&StagedAdd::Stressor {
            description: "queue overload".into(),
            attractor_id: "A-01".into(),
            naive_change: "add retry".into(),
            shortname: String::new(),
            outcomes: String::new(),
        });

        assert!(cmd.starts_with("residual add stressor "), "got: {cmd}");
        assert!(cmd.contains("--description 'queue overload'"), "got: {cmd}");
        assert!(
            !cmd.contains("--shortname"),
            "blank --shortname must be omitted: {cmd}"
        );
        assert!(
            !cmd.contains("--outcomes"),
            "blank --outcomes must be omitted: {cmd}"
        );
    }

    /// Required flags are emitted even when the operator left them blank, so the
    /// pasted command fails loudly in clap rather than silently doing the wrong thing.
    #[test]
    fn command_generator_keeps_required_flags_when_blank() {
        let cmd = format_add_command(&StagedAdd::Stressor {
            description: String::new(),
            attractor_id: String::new(),
            naive_change: String::new(),
            shortname: String::new(),
            outcomes: String::new(),
        });
        for flag in ["--description", "--attractor-id", "--naive-change"] {
            assert!(
                cmd.contains(flag),
                "required flag {flag} must always be emitted, got: {cmd}"
            );
        }
    }

    #[test]
    fn command_generator_emits_valid_add_purpose_shape() {
        let cmd = format_add_command(&StagedAdd::Purpose {
            description: "operator sees the landscape".into(),
            attractor_id: "A-02".into(),
            naive_change: "ship a view command".into(),
            shortname: "landscape-view".into(),
            outcomes: "operator filters forces".into(),
        });

        assert!(cmd.starts_with("residual add purpose "), "got: {cmd}");
        for flag in [
            "--description 'operator sees the landscape'",
            "--attractor-id 'A-02'",
            "--naive-change 'ship a view command'",
            "--shortname 'landscape-view'",
            "--outcomes 'operator filters forces'",
        ] {
            assert!(cmd.contains(flag), "missing {flag} in: {cmd}");
        }
        assert!(
            !cmd.contains("--feature"),
            "emit the canonical --naive-change flag, not the --feature alias: {cmd}"
        );
    }

    #[test]
    fn command_generator_emits_valid_add_attractor_shape() {
        let cmd = format_add_command(&StagedAdd::Attractor {
            name: "Stability".into(),
            description: "System remains stable".into(),
            positive_state: "coherent NKP".into(),
            negative_state: "Ri collapses".into(),
        });

        assert!(cmd.starts_with("residual add attractor "), "got: {cmd}");
        for flag in [
            "--name 'Stability'",
            "--description 'System remains stable'",
            "--positive-state 'coherent NKP'",
            "--negative-state 'Ri collapses'",
        ] {
            assert!(cmd.contains(flag), "missing {flag} in: {cmd}");
        }
    }

    #[test]
    fn command_generator_emits_valid_add_component_shape() {
        let cmd = format_add_command(&StagedAdd::Component {
            name: "view".into(),
            description: "Landscape view".into(),
            status: "proposed".into(),
            architecture_set: "iter5-view".into(),
        });

        assert!(cmd.starts_with("residual add component "), "got: {cmd}");
        for flag in [
            "--name 'view'",
            "--description 'Landscape view'",
            "--status 'proposed'",
            "--architecture-set 'iter5-view'",
        ] {
            assert!(cmd.contains(flag), "missing {flag} in: {cmd}");
        }
    }

    /// Defense-mode staging emits the meta force and defense artifact commands.
    #[test]
    fn command_generator_emits_defense_shapes() {
        let ms = format_add_command(&StagedAdd::MetaStressor {
            description: "Defense-layer stressor".into(),
            shortname: "meta-force-landscape".into(),
        });
        assert!(ms.starts_with("residual add meta-stressor "), "got: {ms}");
        assert!(ms.contains("--description 'Defense-layer stressor'"), "got: {ms}");
        assert!(ms.contains("--shortname 'meta-force-landscape'"), "got: {ms}");
        assert!(
            !ms.contains("--attractor-id"),
            "add meta-stressor has no --attractor-id flag in clap: {ms}"
        );
        assert!(
            !ms.contains("--naive-change"),
            "add meta-stressor has no --naive-change flag in clap: {ms}"
        );

        let ma = format_add_command(&StagedAdd::MetaAttractor {
            name: "Practitioner Standing".into(),
            description: "Defense attractor".into(),
            positive_state: "safe to discuss".into(),
            negative_state: "reads as heresy".into(),
        });
        assert!(ma.starts_with("residual add meta-attractor "), "got: {ma}");
        assert!(ma.contains("--positive-state 'safe to discuss'"), "got: {ma}");

        let mp = format_add_command(&StagedAdd::MetaPurpose {
            description: "Defense purpose".into(),
            attractor_id: "MA-01".into(),
            naive_change: "capture meta forces".into(),
            shortname: "defense-efficacy".into(),
            outcomes: "operator records defense purpose".into(),
        });
        assert!(mp.starts_with("residual add meta-purpose "), "got: {mp}");
        assert!(mp.contains("--attractor-id 'MA-01'"), "got: {mp}");

        for staged in [
            StagedAdd::DefensePersona {
                name: "hostile-auditor".into(),
                body: "# md\n".into(),
            },
            StagedAdd::DefenseStrategy {
                name: "alpha-strategy".into(),
                body: "# md\n".into(),
            },
            StagedAdd::DefenseProgress {
                name: "week-1".into(),
                body: "# md\n".into(),
            },
            StagedAdd::DefensePitch {
                name: "exec-summary".into(),
                body: "# md\n".into(),
            },
        ] {
            let cmd = format_add_command(&staged);
            let expected = format!("residual add {} ", staged.subcommand());
            assert!(cmd.starts_with(&expected), "expected `{expected}…`, got: {cmd}");
            assert!(cmd.contains("--name "), "got: {cmd}");
            assert!(cmd.contains("--body "), "got: {cmd}");
        }
    }

    /// Values with apostrophes must not break out of the single-quoted argument.
    #[test]
    fn command_generator_escapes_single_quotes() {
        let quoted = shell_quote("operator's zag");
        assert_eq!(
            quoted, r#"'operator'\''s zag'"#,
            "single quotes must be POSIX-escaped, got: {quoted}"
        );

        let cmd = format_add_command(&StagedAdd::Component {
            name: "view".into(),
            description: "operator's landscape".into(),
            status: "proposed".into(),
            architecture_set: "iter5-view".into(),
        });
        assert!(
            cmd.contains(r#"--description 'operator'\''s landscape'"#),
            "escaped value must appear in the command, got: {cmd}"
        );
    }

    /// Newlines in a body must not split one command into two shell lines.
    #[test]
    fn command_generator_keeps_multiline_body_in_one_argument() {
        let cmds = format_add_commands(&[StagedAdd::DefenseStrategy {
            name: "alpha-strategy".into(),
            body: "# heading\n\nbody line\n".into(),
        }]);
        assert!(
            cmds.starts_with("residual add defense-strategy "),
            "got: {cmds}"
        );
        assert!(
            cmds.contains("$'# heading\\n\\nbody line\\n'"),
            "multiline body must be emitted as a single-line ANSI-C quoted argument, got: {cmds}"
        );
        assert_eq!(
            cmds.lines().count(),
            1,
            "a multiline body must still produce exactly one command line, got: {cmds}"
        );
    }

    /// The copy surface emits one command per staged add, in staging order.
    #[test]
    fn format_add_commands_is_newline_separated_in_order() {
        let cmds = format_add_commands(&[
            StagedAdd::Attractor {
                name: "Stability".into(),
                description: "d".into(),
                positive_state: "p".into(),
                negative_state: "n".into(),
            },
            stressor(),
        ]);

        let lines: Vec<&str> = cmds.lines().collect();
        assert_eq!(lines.len(), 2, "expected two commands, got: {cmds}");
        assert!(lines[0].starts_with("residual add attractor "), "got: {cmds}");
        assert!(lines[1].starts_with("residual add stressor "), "got: {cmds}");
        assert!(
            !cmds.ends_with('\n'),
            "no trailing newline so the copy buffer pastes cleanly, got: {cmds:?}"
        );
    }

    #[test]
    fn format_add_commands_empty_is_empty_string() {
        assert_eq!(format_add_commands(&[]), "");
    }

    /// Staged adds round-trip through JSON so the browser can post them back verbatim.
    #[test]
    fn staged_add_round_trips_through_json() {
        let json = serde_json::to_string(&stressor()).expect("serialize staged add");
        assert!(
            json.contains("\"target\":\"stressor\""),
            "staged add must carry a kebab-case target tag, got: {json}"
        );
        let back: StagedAdd = serde_json::from_str(&json).expect("deserialize staged add");
        assert_eq!(back, stressor());
    }

    #[test]
    fn defense_variants_are_flagged_as_defense() {
        assert!(!stressor().is_defense());
        assert!(StagedAdd::MetaStressor {
            description: "d".into(),
            shortname: String::new(),
        }
        .is_defense());
        assert!(StagedAdd::DefensePitch {
            name: "exec-summary".into(),
            body: "b".into(),
        }
        .is_defense());
    }
}
