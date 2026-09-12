#!/usr/bin/env bash
# Regenerates web/generated/cli-schema.json from the real clap Command tree.
# Run inside `nix develop` (or wherever a fresh `residual` binary is on PATH).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --quiet
./target/debug/residual internal cli-schema > web/generated/cli-schema.json
echo "wrote web/generated/cli-schema.json"
