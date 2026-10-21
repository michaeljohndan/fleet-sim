#!/usr/bin/env bash
# Installs a git pre-commit hook that refuses commits unless scripts/check.sh passes.
set -euo pipefail
cd "$(dirname "$0")/.."
cat > .git/hooks/pre-commit << 'HOOK'
#!/usr/bin/env bash
exec scripts/check.sh
HOOK
chmod +x .git/hooks/pre-commit
echo "pre-commit hook installed: every commit now runs scripts/check.sh"
