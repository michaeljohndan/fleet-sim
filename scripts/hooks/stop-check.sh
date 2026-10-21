#!/usr/bin/env bash
# Claude Code Stop hook: before the agent finishes a turn, run the test gate.
# Exit 2 sends the failure back to the agent so it keeps working instead of stopping.
# If the agent is already continuing because of this hook, report but do not loop forever.
input="$(cat)"
cd "$(dirname "$0")/../.."
if out="$(scripts/check.sh 2>&1)"; then
  exit 0
fi
if grep -q '"stop_hook_active": *true' <<<"$input"; then
  echo "check.sh still failing; tell the engineer which iteration broke it." >&2
  exit 0
fi
echo "scripts/check.sh failed. Fix it or revert before finishing:" >&2
tail -n 40 <<<"$out" >&2
exit 2
