#!/usr/bin/env bash
# Installs OneCorp from this checkout: the `onecorp` command and the OneCorp app.
#
#   scripts/install.sh            app goes to ~/Applications
#   scripts/install.sh --system   app goes to /Applications
#   scripts/install.sh --cli      only the command, no app
#
# Needs Rust (rustup.rs). The app build also needs Node.js (nodejs.org) for its UI, and the Tauri
# CLI, installed on first run.
set -euo pipefail

cd "$(dirname "$0")/.."
apps="${ONECORP_APPS_DIR:-$HOME/Applications}"
with_app=1
for arg in "$@"; do
  case "$arg" in
    --system) apps="/Applications" ;;
    --cli) with_app=0 ;;
    *) echo "Unknown option: $arg. Use --system or --cli." >&2; exit 1 ;;
  esac
done

say() { printf '\n\033[1m%s\033[0m\n' "$1"; }

command -v cargo >/dev/null || { echo "Rust isn't installed. Get it from https://rustup.rs, then run this again." >&2; exit 1; }

say "Installing the onecorp command"
cargo install --path crates/cli --locked --quiet
# Where cargo install puts binaries, honouring the usual overrides.
bin="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to use onecorp from anywhere." ;;
esac

if [ "$with_app" = 1 ]; then
  command -v npm >/dev/null || { echo "Node.js isn't installed, and the app's UI needs it to build. Get it from https://nodejs.org, then run this again." >&2; exit 1; }
  say "Installing the UI's packages"
  (cd app && npm ci --silent)
  if ! cargo tauri --version >/dev/null 2>&1; then
    say "Installing the Tauri CLI (first time only, takes a few minutes)"
    cargo install tauri-cli --version "^2" --locked --quiet
  fi
  say "Building OneCorp.app"
  (cd app && cargo tauri build --bundles app)
  built="target/release/bundle/macos/OneCorp.app"
  [ -d "$built" ] || { echo "The build finished but $built is missing." >&2; exit 1; }
  mkdir -p "$apps"
  rm -rf "$apps/OneCorp.app"
  cp -R "$built" "$apps/"
  echo "Installed $apps/OneCorp.app. Open it from Spotlight or the Dock."
fi

say "Checking your setup"
"$bin/onecorp" doctor || true
