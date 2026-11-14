#!/bin/sh
# Build the Nesticles Alpha 2 Phase 9.1 test-candidate package.
# Usage: packaging/phase91/build-linux.sh [output-directory]
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
cargo build --release -p nesticles --offline
STAGE=$(mktemp -d)
NAME=Nesticles-Alpha2-Phase9.1-linux-x86_64
DIR="$STAGE/$NAME"
mkdir -p "$DIR"
cp -f target/release/nesticles "$DIR/nesticles"
cp -f crates/nesticles/assets/icon.png "$DIR/icon.png"
cp -f packaging/phase91/QUICKSTART.md "$DIR/QUICKSTART.md"
cp -f packaging/phase91/LIMITATIONS.md "$DIR/LIMITATIONS.md"
cp -f packaging/phase91/README.md "$DIR/README.md"
cp -f packaging/phase91/API.md "$DIR/API.md"
cp -f packaging/phase91/ROM.md "$DIR/ROM.md"
cp -f packaging/phase91/KNOWLEDGE.md "$DIR/KNOWLEDGE.md"
cp -f packaging/phase91/install.sh "$DIR/install.sh"
cp -f packaging/phase91/uninstall.sh "$DIR/uninstall.sh"
chmod 755 "$DIR/nesticles" "$DIR/install.sh" "$DIR/uninstall.sh"
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
cat > "$DIR/VERSION" <<EOF
Nesticles Alpha 2 — Phase 9.1 test candidate
version 0.1.0
commit ${COMMIT}
platform linux-x86_64
This is not a final accepted Phase 9.1 build.
EOF
OUT=${1:-$ROOT/dist}
mkdir -p "$OUT"
tar -C "$STAGE" -czf "$OUT/$NAME.tar.gz" "$NAME"
(
    cd "$OUT"
    sha256sum "$NAME.tar.gz" | tee "$NAME.tar.gz.sha256"
)
echo "Package: $OUT/$NAME.tar.gz"
rm -rf "$STAGE"
