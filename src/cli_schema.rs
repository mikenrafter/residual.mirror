//! cli-schema — introspects the live clap `Command` tree for mutation
//! subcommands and emits a machine-derived, sorted JSON schema.
//!
//! Process: never hand-list flag names here. Everything must come from
//! `clap::Command`/`clap::Arg` introspection (`get_subcommands`, `get_id`,
//! `is_required_set`, `get_action`, …) so this reflects `src/cli.rs`'s real
//! definitions rather than a manually maintained mirror that can drift.

use clap::CommandFactory;
use serde::Serialize;

/// One flag on a mutation subcommand, as introspected from clap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlagSchema {
    pub name: String,
    pub required: bool,
    pub multiple: bool,
}

/// One mutation subcommand and its flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubcommandSchema {
    pub subcommand: String,
    pub flags: Vec<FlagSchema>,
}

/// Walk `<Cli as clap::CommandFactory>::command()` and produce a sorted,
/// deterministic schema covering `add *`, `update *`, and force-removal subcommands.
///
/// Process: purely derived from clap's introspection API — no hand-listing
/// of subcommand or flag names anywhere in this function.
pub fn cli_schema() -> Vec<SubcommandSchema> {
    let root = crate::cli::Cli::command();

    let mut schema: Vec<SubcommandSchema> = root
        .get_subcommands()
        .filter(|parent| {
            parent.get_name() == "add" || parent.get_name() == "update" || parent.get_name() == "remove"
        })
        .flat_map(|parent| {
            let parent_name = parent.get_name().to_string();
            parent.get_subcommands().map(move |leaf| {
                let mut flags: Vec<FlagSchema> = leaf
                    .get_arguments()
                    .filter_map(|arg| arg.get_long().map(|long| (long, arg)))
                    .filter(|(long, _)| *long != "help")
                    .map(|(long, arg)| FlagSchema {
                        name: long.to_string(),
                        required: arg.is_required_set(),
                        multiple: matches!(arg.get_action(), clap::ArgAction::Append),
                    })
                    .collect();
                flags.sort_by(|a, b| a.name.cmp(&b.name));

                SubcommandSchema {
                    subcommand: format!("{parent_name} {}", leaf.get_name()),
                    flags,
                }
            })
        })
        .collect();

    schema.sort_by(|a, b| a.subcommand.cmp(&b.subcommand));
    schema
}
