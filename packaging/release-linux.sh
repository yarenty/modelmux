#!/usr/bin/env bash
# Build .deb packages for Ubuntu/Debian (for local testing or manual upload).
# Full release: push a tag (v*.*.*) — GitHub Actions builds binaries + .deb and publishes to Releases.
#
# Usage:
#   ./packaging/release-linux.sh              # Build .deb for current arch
#   ./packaging/release-linux.sh --all        # Build for amd64 + arm64 (needs cross-compile)
#
# Prerequisites: cargo-deb, rustup target for desired arch
#   cargo install cargo-deb
#   rustup target add x86_64-unknown-linux-gnu   # amd64
#   rustup target add aarch64-unknown-linux-gnu # arm64

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

build_deb() {
  local target="$1"
  echo "Building .deb for $target..."
  cargo build --release --target "$target"
  cargo deb --no-build --target "$target"
}

if [[ "${1:-}" == "--all" ]]; then
  build_deb x86_64-unknown-linux-gnu
  build_deb aarch64-unknown-linux-gnu
  echo ""
  echo "Built packages:"
  ls -la target/debian/*.deb
else
  # On Linux use host; on macOS/other use amd64 (requires: rustup target add x86_64-unknown-linux-gnu)
  HOST=$(rustc -vV 2>/dev/null | sed -n 's/host: //p')
  if [[ "$HOST" == *-linux-* ]]; then
    TARGET="$HOST"
  else
    TARGET="x86_64-unknown-linux-gnu"
    echo "Not on Linux — building for $TARGET (install: rustup target add $TARGET)"
  fi
  build_deb "$TARGET"
  echo ""
  echo "Built: $(ls target/debian/*.deb)"
  echo ""
  echo "Install: sudo dpkg -i target/debian/*.deb"
  echo ""
  echo "Manual systemd setup (if not using .deb):"
  echo "  1. sudo cp target/release/modelmux /usr/bin/"
  echo "  2. sudo mkdir -p /etc/modelmux"
  echo "  3. sudo cp packaging/systemd/config.toml.example /etc/modelmux/config.toml"
  echo "  4. sudo vi /etc/modelmux/config.toml  # Edit with your GCP settings"
  echo "  5. sudo cp packaging/systemd/modelmux.service /etc/systemd/system/"
  echo "  6. sudo systemctl daemon-reload"
  echo "  7. sudo systemctl enable --now modelmux"
  echo ""
  echo "Verify: modelmux doctor && sudo systemctl status modelmux"
fi
