#![cfg_attr(test, allow(clippy::unwrap_used))]
use anyhow::Result;

mod cli;
mod cli_schema;
mod config;
mod nkp;
mod skills;
mod storage;
mod structure;
mod tags;
mod verification;
mod verify;
mod view;

fn main() -> Result<()> {
    cli::run()
}
