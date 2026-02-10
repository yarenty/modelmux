#!/usr/bin/env bash
# Release modelmux formula to yarenty/homebrew-tap
# Run from project root or packaging/ after updating packaging/homebrew/modelmux.rb

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FORMULA_SRC="$PROJECT_ROOT/packaging/homebrew/modelmux.rb"

# Resolve tap path (brew tap yarenty/tap -> homebrew-tap)
TAP_PATH="$(brew --repository yarenty/tap 2>/dev/null)" || true
if [[ -z "$TAP_PATH" || ! -d "$TAP_PATH" ]]; then
  echo "Error: tap yarenty/tap not found. Run: brew tap yarenty/tap"
  exit 1
fi

FORMULA_DEST="$TAP_PATH/Formula/modelmux.rb"
mkdir -p "$(dirname "$FORMULA_DEST")"

# Extract version from formula for commit message (e.g. url ".../v0.5.0.tar.gz" -> 0.5.0)
VERSION=$(grep 'url "' "$FORMULA_SRC" | sed -n 's/.*v\([0-9][0-9.]*\)\.tar\.gz.*/\1/p' | head -1)
[[ -z "$VERSION" ]] && VERSION="update"

echo "Copying $FORMULA_SRC -> $FORMULA_DEST"
cp "$FORMULA_SRC" "$FORMULA_DEST"

cd "$TAP_PATH"

if ! git diff --quiet Formula/modelmux.rb 2>/dev/null; then
  git add Formula/modelmux.rb
  git commit -m "modelmux: update to v${VERSION}"
  git push
  echo "Done. Pushed modelmux v${VERSION} to yarenty/tap"
else
  echo "No changes to Formula/modelmux.rb - nothing to commit"
fi
