#!/usr/bin/env bash
# Find the first commit that broke the build or the tests.
# Usage: scripts/bisect.sh <known-good-commit> [known-bad-commit, default HEAD]
# Each commit is tested with scripts/check.sh --fast; git reports the first bad one.
set -euo pipefail
cd "$(dirname "$0")/.."
good="${1:?usage: scripts/bisect.sh <good-commit> [bad-commit]}"
bad="${2:-HEAD}"
# Copy the checker outside the tree so older commits use today's checker.
tmp="$(mktemp)"; cp scripts/check.sh "$tmp"; chmod +x "$tmp"
git bisect start "$bad" "$good"
git bisect run "$tmp" --fast || true
git bisect log | tail -n 5
git bisect reset
rm -f "$tmp"
