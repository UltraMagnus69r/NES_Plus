#!/usr/bin/env bash
# Nesticles R3 native desktop build/run for Bazzite + Distrobox.
# Run with: bash run-r3.sh
# Do not use `set -e` in an interactive Konsole shell.
set -uo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ASSETS_DIR="${NESTICLES_ASSETS_DIR:-$HOME/Desktop/Nesticle/Assets}"
LOG="${NESTICLES_BUILD_LOG:-$HOME/Desktop/nesticles-r3-build.log}"
RUNLOG="${NESTICLES_RUN_LOG:-$HOME/Desktop/nesticles-r3-runtime.log}"
BOX="${NESTICLES_DISTROBOX:-nesticles-visual-dev}"

printf '\nNesticles R3 – build from source\nSource: %s\n' "$ROOT"
mkdir -p "$ASSETS_DIR"
# Bundle files are already included in the binary; only install non-conflicting
# copies for the explicit user Assets folder override workflow.
cp -a -n "$ROOT/crates/nesticles/assets/ui/." "$ASSETS_DIR/" 2>/dev/null || true
printf 'Assets: %s\nBuild log: %s\n' "$ASSETS_DIR" "$LOG"

if command -v distrobox >/dev/null 2>&1; then
    printf 'Building inside existing Distrobox: %s\n' "$BOX"
    distrobox enter "$BOX" -- bash -c '
      cd "$1" || exit 30
      cargo build --release --locked -p nesticles
    ' bash "$ROOT" > "$LOG" 2>&1
    result=$?
else
    printf 'No Distrobox command found; attempting host Cargo build.\n'
    (cd "$ROOT" && cargo build --release --locked -p nesticles) > "$LOG" 2>&1
    result=$?
fi
if [ "$result" -ne 0 ]; then
    printf '\nBUILD FAILED (exit %s). Last 60 log lines:\n' "$result"
    tail -n 60 "$LOG"
    printf '\nYour terminal stays open. Share %s for a source fix.\n' "$LOG"
    exit "$result"
fi
printf '\nBUILD PASSED. Launching Nesticles R3.\n'
if command -v distrobox >/dev/null 2>&1; then
    distrobox enter "$BOX" -- bash -c '
      cd "$1" || exit 30
      exec ./target/release/nesticles
    ' bash "$ROOT" > "$RUNLOG" 2>&1
    result=$?
else
    (cd "$ROOT" && ./target/release/nesticles) > "$RUNLOG" 2>&1
    result=$?
fi
if [ "$result" -ne 0 ]; then
    printf 'Nesticles exited with code %s. Last 50 log lines:\n' "$result"
    tail -n 50 "$RUNLOG"
fi
exit "$result"
