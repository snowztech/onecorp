#!/usr/bin/env bash
# Builds the drag-to-Applications disk image: OneCorp next to a link to /Applications, in a small
# window with no toolbar. The layout comes from app/dmg/DS_Store, so Finder never has to be scripted,
# which doesn't work on a CI runner. Signed when APPLE_SIGNING_IDENTITY is set.
#
#   scripts/dmg.sh path/to/OneCorp.app path/to/OneCorp.dmg
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
app="$1"
dmg="$2"
[ -d "$app" ] || { echo "No app at $app. Build it first." >&2; exit 1; }

staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
cp -R "$app" "$staging/OneCorp.app"
ln -s /Applications "$staging/Applications"
# Window size and icon positions, made once with dmgbuild. Finder matches them by item name.
cp "$root/app/dmg/DS_Store" "$staging/.DS_Store"

mkdir -p "$(dirname "$dmg")"
rm -f "$dmg"
hdiutil create -quiet -volname OneCorp -srcfolder "$staging" -ov -format UDZO "$dmg"
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$dmg"
fi
echo "Built $dmg"
