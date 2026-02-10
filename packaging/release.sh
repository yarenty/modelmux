#!/usr/bin/env bash
# Release modelmux formula to yarenty/homebrew-tap
# 1. Reads version from Cargo.toml
# 2. Fetches tarball from GitHub, computes sha256
# 3. Updates modelmux.rb (url, sha256)
# 4. Copies to tap, commits, pushes
# Run from project root: ./packaging/release.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
FORMULA_SRC="$PROJECT_ROOT/packaging/homebrew/modelmux.rb"
REPO="yarenty/modelmux"

# 1. Get version from Cargo.toml
VERSION=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\([^"]*\)"/\1/')
if [[ -z "$VERSION" ]]; then
  echo "Error: could not extract version from Cargo.toml"
  exit 1
fi
echo "Version: $VERSION"

# 2. Fetch tarball and compute sha256
TARBALL_URL="https://github.com/${REPO}/archive/refs/tags/v${VERSION}.tar.gz"
TARBALL="/tmp/modelmux-v${VERSION}.tar.gz"
echo "Fetching $TARBALL_URL"
HTTP_CODE=$(curl -sL -o "$TARBALL" -w "%{http_code}" "$TARBALL_URL")
if [[ "$HTTP_CODE" != "200" ]]; then
  echo "Error: Failed to fetch tarball (HTTP $HTTP_CODE). Is the tag v${VERSION} published?"
  exit 1
fi
SHA256=$(shasum -a 256 "$TARBALL" | awk '{print $1}')
echo "sha256: $SHA256"

# 3. Update modelmux.rb (url and sha256)
sed -i.bak \
  -e "s|url \"https://github.com/${REPO}/archive/refs/tags/v[0-9.]*\.tar\.gz\"|url \"${TARBALL_URL}\"|" \
  -e "s|sha256 \"[a-f0-9]*\"|sha256 \"${SHA256}\"|" \
  "$FORMULA_SRC"
rm -f "${FORMULA_SRC}.bak"

# 4. Resolve tap path and copy
TAP_PATH="$(brew --repository yarenty/tap 2>/dev/null)" || true
if [[ -z "$TAP_PATH" || ! -d "$TAP_PATH" ]]; then
  echo "Error: tap yarenty/tap not found. Run: brew tap yarenty/tap"
  exit 1
fi

FORMULA_DEST="$TAP_PATH/Formula/modelmux.rb"
mkdir -p "$(dirname "$FORMULA_DEST")"
echo "Copying $FORMULA_SRC -> $FORMULA_DEST"
cp "$FORMULA_SRC" "$FORMULA_DEST"

# 5. Git add, commit, push
cd "$TAP_PATH"
if ! git diff --quiet Formula/modelmux.rb 2>/dev/null; then
  git add Formula/modelmux.rb
  git commit -m "modelmux: update to v${VERSION}"
  git push
  echo "Done. Pushed modelmux v${VERSION} to yarenty/tap"
else
  echo "No changes to Formula/modelmux.rb - nothing to commit"
fi
