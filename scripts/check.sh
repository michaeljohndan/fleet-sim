#!/usr/bin/env bash
# The single quality gate. Every iteration must pass this before it is committed.
# Usage: scripts/check.sh            (full: fmt, clippy, tests)
#        scripts/check.sh --fast     (tests only; used by git bisect)
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" != "--fast" ]]; then
  echo "==> cargo fmt --check"
  cargo fmt --all -- --check
  echo "==> cargo clippy (warnings are errors)"
  cargo clippy --all-targets --quiet -- -D warnings
fi
echo "==> cargo test"
cargo test --quiet
echo "==> all checks passed"
