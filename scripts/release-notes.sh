#!/usr/bin/env bash
# Prints one version's notes from CHANGELOG.md, for a GitHub release: everything under
# "## 0.1.0" up to the next "## ". Falls back to "## Unreleased" when that version has no section yet.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
section() {
  awk -v want="$1" '
    /^## / { printing = ($0 == "## " want); next }
    printing { print }
  ' "$root/CHANGELOG.md" | sed -e '/./,$!d'
}
notes=$(section "$1")
[ -n "$notes" ] || notes=$(section "Unreleased")
printf '%s\n' "$notes"
