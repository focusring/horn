#!/usr/bin/env bash
set -euo pipefail

# Verify that every version string in the repository matches the workspace
# version. Optionally pass an expected version (e.g. the release tag without
# the "v" prefix) as the first argument.
#
#   scripts/check-version.sh            # all files agree with Cargo.toml
#   scripts/check-version.sh 0.3.0      # ... and that version is 0.3.0

cd "$(dirname "$0")/.."

WORKSPACE_VERSION=$(cargo metadata --format-version 1 --no-deps \
  | jq -r '.packages[] | select(.name == "horn") | .version')
EXPECTED=${1:-$WORKSPACE_VERSION}

status=0
check() {
  local what=$1 actual=$2
  if [ "$actual" = "$EXPECTED" ]; then
    echo "ok       $what = $actual"
  else
    echo "MISMATCH $what = '$actual' (expected $EXPECTED)"
    status=1
  fi
}

check "Cargo.toml [workspace.package]" "$WORKSPACE_VERSION"
while IFS=$'\t' read -r name version; do
  check "crate $name" "$version"
done < <(cargo metadata --format-version 1 --no-deps \
  | jq -r '.packages[] | [.name, .version] | @tsv')

check "horn-desktop/src-tauri/tauri.conf.json" "$(jq -r .version horn-desktop/src-tauri/tauri.conf.json)"
check "docs/package.json" "$(jq -r .version docs/package.json)"
for f in horn-desktop/src/index.html horn-gui/static/index.html; do
  check "$f footer" "$(sed -nE 's/.*Horn v([0-9]+\.[0-9]+\.[0-9]+[^ ]*) —.*/\1/p' "$f" | head -n1)"
done

if [ "$status" -ne 0 ]; then
  echo
  echo "Run scripts/sync-version.sh to propagate the workspace version." >&2
fi
exit $status
