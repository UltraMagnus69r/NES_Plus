#!/bin/sh
# Install the Phase 9.1 test candidate for the current user. No administrator rights.
# This does not replace the Alpha 2 Phase 9 install.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/nesticles-alpha2-phase9-1"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$DEST" "$APP_DIR"
cp -f "$ROOT/nesticles" "$DEST/nesticles"
cp -f "$ROOT/icon.png" "$DEST/icon.png"
for doc in QUICKSTART.md LIMITATIONS.md README.md API.md ROM.md KNOWLEDGE.md VERSION; do
    cp -f "$ROOT/$doc" "$DEST/$doc"
done
chmod 755 "$DEST/nesticles"
DESKTOP_FILE="$APP_DIR/nesticles-alpha2-phase9-1.desktop"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Nesticles Alpha 2.1 Test Candidate
GenericName=Game workshop
Comment=Extract NES graphics, review them, and test a course
Exec=$DEST/nesticles
Icon=$DEST/icon.png
Path=$DEST
Terminal=false
Categories=Game;Development;
StartupWMClass=nesticles
EOF
chmod 644 "$DESKTOP_FILE"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
DESKTOP_DIR=""
if command -v xdg-user-dir >/dev/null 2>&1; then
    DESKTOP_DIR=$(xdg-user-dir DESKTOP 2>/dev/null || true)
fi
if [ -z "$DESKTOP_DIR" ] || [ ! -d "$DESKTOP_DIR" ]; then
    if [ -d "$HOME/Desktop" ]; then
        DESKTOP_DIR="$HOME/Desktop"
    fi
fi
if [ -n "$DESKTOP_DIR" ] && [ -d "$DESKTOP_DIR" ]; then
    cp -f "$DESKTOP_FILE" "$DESKTOP_DIR/Nesticles Alpha 2.1 Test Candidate.desktop"
    chmod 755 "$DESKTOP_DIR/Nesticles Alpha 2.1 Test Candidate.desktop"
    echo "Desktop launcher: $DESKTOP_DIR/Nesticles Alpha 2.1 Test Candidate.desktop"
fi
echo "Installed: $DEST/nesticles"
echo "Menu entry: $DESKTOP_FILE"
echo "Alpha 2 Phase 9, if installed, was left in place."
