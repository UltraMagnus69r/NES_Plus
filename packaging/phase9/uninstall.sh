#!/bin/sh
# Remove the user-local Nesticles Alpha 2 Phase 9 install. Your projects are not deleted.
set -eu
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/nesticles-alpha2-phase9"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
rm -f "$APP_DIR/nesticles-alpha2-phase9.desktop"
DESKTOP_DIR=""
if command -v xdg-user-dir >/dev/null 2>&1; then
    DESKTOP_DIR=$(xdg-user-dir DESKTOP 2>/dev/null || true)
fi
if [ -z "$DESKTOP_DIR" ] || [ ! -d "$DESKTOP_DIR" ]; then
    DESKTOP_DIR="$HOME/Desktop"
fi
rm -f "$DESKTOP_DIR/Nesticles Alpha 2.desktop"
rm -rf "$DEST"
echo "Removed Nesticles Alpha 2 Phase 9 from $DEST"
echo "Project JSON files you saved elsewhere were left in place."
