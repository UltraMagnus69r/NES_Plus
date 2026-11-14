#!/bin/sh
# Remove the Phase 9.1 test candidate. Saved projects and the Phase 9 install stay.
set -eu
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/nesticles-alpha2-phase9-1"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
rm -f "$APP_DIR/nesticles-alpha2-phase9-1.desktop"
DESKTOP_DIR=""
if command -v xdg-user-dir >/dev/null 2>&1; then
    DESKTOP_DIR=$(xdg-user-dir DESKTOP 2>/dev/null || true)
fi
if [ -z "$DESKTOP_DIR" ] || [ ! -d "$DESKTOP_DIR" ]; then
    DESKTOP_DIR="$HOME/Desktop"
fi
rm -f "$DESKTOP_DIR/Nesticles Alpha 2.1 Test Candidate.desktop"
rm -rf "$DEST"
echo "Removed the Phase 9.1 test candidate from $DEST"
echo "Project files, exported packages, and the Phase 9 install were left in place."
echo "The alpha library under ~/.local/share/nesticles/nesticles/alpha2-phase9.1 was left in place."
echo "An API key in Secret Service was left in place."
