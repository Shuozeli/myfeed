#!/bin/bash
# Run a site exploration recipe and save the output.
#
# Usage:
#   ./scripts/explore.sh <site> [cdp_endpoint]
#
# Examples:
#   ./scripts/explore.sh 1point3acres ws://localhost:9222
#   ./scripts/explore.sh reddit ws://localhost:9222

set -euo pipefail

SITE="${1:?usage: explore.sh <site> [cdp_endpoint]}"
CDP="${2:-ws://localhost:9222}"
RECIPE="recipes/explore/${SITE}-explore.yaml"
OUTDIR="docs/private"

if [ ! -f "$RECIPE" ]; then
  echo "Recipe not found: $RECIPE"
  exit 1
fi

mkdir -p "$OUTDIR"

echo "Running exploration: $RECIPE (CDP: $CDP)"
CDP_ENDPOINT="$CDP" \
  pwright script run "$RECIPE" \
  > "${OUTDIR}/${SITE}-explore-output.jsonl" 2>&1

echo "Output saved to ${OUTDIR}/${SITE}-explore-output.jsonl"
echo "Parse the structure field for site analysis."
