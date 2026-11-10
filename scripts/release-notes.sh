#!/usr/bin/env bash
# Prints one version's notes from CHANGELOG.md, for a GitHub release: everything under
# "## [0.1.0] - date" up to the next version, without the link lines at the bottom.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
awk -v version="$1" '
  /^## \[/ { printing = index($0, "## [" version "]") == 1; next }
  /^\[.*\]: / { next }
  printing { print }
' "$root/CHANGELOG.md" | sed -e '/./,$!d'
