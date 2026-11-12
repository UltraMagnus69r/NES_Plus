#!/bin/sh
# Build the Nesticles Alpha 1 UI refresh portable Linux package.
# Usage: packaging/ui-refresh/build-linux.sh [output-directory]
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
cargo build --release -p nesticles --offline
STAGE=$(mktemp -d)
NAME=Nesticles-Alpha1-UI-linux-x86_64
DIR="$STAGE/$NAME"
mkdir -p "$DIR"
cp -f target/release/nesticles "$DIR/nesticles"
cp -f crates/nesticles/assets/icon.png "$DIR/icon.png"
cp -f packaging/alpha1/QUICKSTART.md "$DIR/QUICKSTART.md"
cp -f packaging/alpha1/LIMITATIONS.md "$DIR/LIMITATIONS.md"
cp -f packaging/ui-refresh/README.md "$DIR/README.md"
cp -f packaging/ui-refresh/install.sh "$DIR/install.sh"
cp -f packaging/ui-refresh/uninstall.sh "$DIR/uninstall.sh"
chmod 755 "$DIR/nesticles" "$DIR/install.sh" "$DIR/uninstall.sh"
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
cat > "$DIR/VERSION" <<EOF
Nesticles Alpha 1 — UI Refresh
version 0.1.0
commit ${COMMIT}
platform linux-x86_64
EOF
OUT=${1:-$ROOT/dist}
mkdir -p "$OUT"
tar -C "$STAGE" -czf "$OUT/$NAME.tar.gz" "$NAME"
sha256sum "$OUT/$NAME.tar.gz" | tee "$OUT/$NAME.tar.gz.sha256"
echo "Package: $OUT/$NAME.tar.gz"
rm -rf "$STAGE"
