#!/usr/bin/env bash
# Points snowztech/homebrew-tap's onecorp cask at a published release, so
# `brew install --cask snowztech/tap/onecorp` installs it. Creates the cask the first time.
set -euo pipefail

version="$1"
sha=$(shasum -a 256 "$2" | cut -d' ' -f1)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if [ -n "${TAP_DEPLOY_KEY:-}" ]; then
  (umask 077 && echo "$TAP_DEPLOY_KEY" > "$work/key")
  export GIT_SSH_COMMAND="ssh -i $work/key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new"
fi

tap="$work/tap"
git clone -q --depth 1 git@github.com:snowztech/homebrew-tap.git "$tap"
cask="$tap/Casks/onecorp.rb"
mkdir -p "$tap/Casks"
cat > "$cask" <<RUBY
cask "onecorp" do
  version "$version"
  sha256 "$sha"

  url "https://github.com/snowztech/onecorp/releases/download/v#{version}/OneCorp.dmg"
  name "OneCorp"
  desc "Run a company of AI employees on your Mac"
  homepage "https://onecorp.dev"

  # The app updates itself, so brew doesn't treat a newer installed version as out of date.
  auto_updates true

  app "OneCorp.app"

  zap trash: [
    "~/.onecorp",
    "~/Library/WebKit/dev.onecorp.desktop",
    "~/Library/Caches/dev.onecorp.desktop",
  ]
end
RUBY

git -C "$tap" add Casks/onecorp.rb
if git -C "$tap" diff --cached --quiet; then
  echo "cask is already at $version"
  exit 0
fi
git -C "$tap" -c user.name="github-actions[bot]" -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
  commit -qm "chore: bump onecorp to $version"
git -C "$tap" push -q
echo "cask bumped to $version"
