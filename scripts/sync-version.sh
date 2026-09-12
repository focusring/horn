#!/usr/bin/env bash
set -euo pipefail

# Propagate the workspace version ([workspace.package].version in Cargo.toml)
# into every non-Cargo file that carries a version string.
#
# Called by cargo-release as a pre-release hook, and safe to run by hand after
# bumping the version manually. Run scripts/check-version.sh to verify that
# everything is in sync without modifying anything.

cd "$(dirname "$0")/.."

VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name == "horn") | .version')

# Tauri bundle version (drives installer file names such as Horn_0.3.0_amd64.deb)
TAURI_CONF="horn-desktop/src-tauri/tauri.conf.json"
jq --arg v "$VERSION" '.version = $v' "$TAURI_CONF" > "$TAURI_CONF.tmp" && mv "$TAURI_CONF.tmp" "$TAURI_CONF"

# Docs site (VitePress nav shows the version from package.json)
DOCS_PKG="docs/package.json"
jq --arg v "$VERSION" '.version = $v' "$DOCS_PKG" > "$DOCS_PKG.tmp" && mv "$DOCS_PKG.tmp" "$DOCS_PKG"

# Footer of the desktop and web GUI front-ends
FOOTERS=(horn-desktop/src/index.html horn-gui/static/index.html)
for f in "${FOOTERS[@]}"; do
  sed -i.bak -E "s/Horn v[0-9]+\.[0-9]+\.[0-9]+[^ ]* —/Horn v${VERSION} —/" "$f" && rm -f "$f.bak"
done

git add "$TAURI_CONF" "$DOCS_PKG" "${FOOTERS[@]}"
