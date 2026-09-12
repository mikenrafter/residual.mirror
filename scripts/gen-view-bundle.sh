#!/usr/bin/env bash
# Regenerates web/generated/app.js from web/src/main.ts.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
cd web && bun build src/main.ts --outfile=generated/app.js --target=browser --format=esm
echo "wrote web/generated/app.js"
