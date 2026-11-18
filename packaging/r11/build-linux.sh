#!/bin/sh
# Build the R1.1 desktop-window test candidate. It does not replace earlier package names.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
export CARGO_TARGET_DIR="$ROOT/target"
cargo build --release -p nesticles --offline
STAGE=$(mktemp -d)
NAME=Nesticles-R1.1-DesktopWindow-linux-x86_64
DIR="$STAGE/$NAME"
mkdir -p "$DIR"
cp -f target/release/nesticles "$DIR/nesticles"
cp -f crates/nesticles/assets/icon.png "$DIR/icon.png"
cp -f packaging/r11/README.md "$DIR/README.md"
cp -f packaging/r11/LIMITATIONS.md "$DIR/LIMITATIONS.md"
cp -f packaging/phase91/API.md "$DIR/API.md"
cp -f packaging/phase91/ROM.md "$DIR/ROM.md"
cp -f packaging/phase91/KNOWLEDGE.md "$DIR/KNOWLEDGE.md"
chmod 755 "$DIR/nesticles"
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
cat > "$DIR/VERSION" <<EOF
Nesticles R1.1 desktop window test candidate
version 0.1.0
commit ${COMMIT}
platform linux-x86_64
R1.1 is not accepted. Phase 9.1 is not accepted. Phase 10 is not included.
The application starts in a decorated window. Fullscreen is optional.
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
