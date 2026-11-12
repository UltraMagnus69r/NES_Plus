#!/bin/sh
# Install Nesticles Alpha 1 for the current user. No administrator rights.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/nesticles-alpha1"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$DEST" "$APP_DIR"
cp -f "$ROOT/nesticles" "$DEST/nesticles"
cp -f "$ROOT/icon.png" "$DEST/icon.png"
cp -f "$ROOT/QUICKSTART.md" "$DEST/QUICKSTART.md"
cp -f "$ROOT/LIMITATIONS.md" "$DEST/LIMITATIONS.md"
cp -f "$ROOT/README.md" "$DEST/README.md"
cp -f "$ROOT/VERSION" "$DEST/VERSION"
chmod 755 "$DEST/nesticles"
DESKTOP_FILE="$APP_DIR/nesticles-alpha1.desktop"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Nesticles Alpha 1
GenericName=Game workshop
Comment=Make a 2D action platformer
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
    cp -f "$DESKTOP_FILE" "$DESKTOP_DIR/Nesticles Alpha 1.desktop"
    chmod 755 "$DESKTOP_DIR/Nesticles Alpha 1.desktop"
    echo "Desktop launcher: $DESKTOP_DIR/Nesticles Alpha 1.desktop"
fi
echo "Installed: $DEST/nesticles"
echo "Menu entry: $DESKTOP_FILE"
echo "Quick start: $DEST/QUICKSTART.md"
