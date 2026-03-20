#!/usr/bin/env bash
set -euo pipefail

# Sync version from Cargo workspace into non-Cargo files.
# Called by cargo-release as a pre-release hook.

VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name == "horn") | .version')

# Update tauri.conf.json
TAURI_CONF="horn-desktop/src-tauri/tauri.conf.json"
jq --arg v "$VERSION" '.version = $v' "$TAURI_CONF" > "$TAURI_CONF.tmp" && mv "$TAURI_CONF.tmp" "$TAURI_CONF"

# Update docs/package.json
DOCS_PKG="docs/package.json"
jq --arg v "$VERSION" '.version = $v' "$DOCS_PKG" > "$DOCS_PKG.tmp" && mv "$DOCS_PKG.tmp" "$DOCS_PKG"

git add "$TAURI_CONF" "$DOCS_PKG"
