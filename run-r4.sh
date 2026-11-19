#!/usr/bin/env bash
# Nesticles R4 - Bazzite native desktop build/run via Distrobox.
# Invoke with `bash run-r4.sh` from a Konsole. This script never enables
# errexit in your interactive shell and never modifies your existing install.
set -uo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ASSETS_DIR="${NESTICLES_ASSETS_DIR:-$HOME/Desktop/Nesticle/Assets}"
LOG="${NESTICLES_BUILD_LOG:-$HOME/Desktop/nesticles-r4-build.log}"
RUNLOG="${NESTICLES_RUN_LOG:-$HOME/Desktop/nesticles-r4-runtime.log}"
BOX="${NESTICLES_DISTROBOX:-nesticles-visual-dev}"
MODE="${1:-}" # --build-only, --run-only or empty

printf '\nNESTICLES R4 — LIVE UI ASSET & CONTROL STYLING\n'
printf 'Source folder: %s\nAssets: %s\nBuild log: %s\n' "$ROOT" "$ASSETS_DIR" "$LOG"
mkdir -p "$ASSETS_DIR" || exit 12
# Compiled-in images always provide a fallback. Existing overrides under the
# user's stated Assets directory win; do not overwrite their custom work.
cp -a -n "$ROOT/crates/nesticles/assets/ui/." "$ASSETS_DIR/" 2>/dev/null || true

if [[ "$MODE" != '--run-only' ]]; then
    printf '\nBuilding native binary (no project or ROM changes)...\n'
    if command -v distrobox >/dev/null 2>&1; then
        distrobox enter "$BOX" -- bash -c '
          export PATH="$HOME/.cargo/bin:$PATH"
          cd "$1" || exit 30
          cargo build --release --locked -p nesticles
        ' bash "$ROOT" > "$LOG" 2>&1
        result=$?
    else
        (cd "$ROOT" && PATH="$HOME/.cargo/bin:$PATH" cargo build --release --locked -p nesticles) > "$LOG" 2>&1
        result=$?
    fi
    if [[ "$result" -ne 0 ]]; then
        printf '\nBUILD FAILED (exit code %s)\n' "$result"
        printf 'Last 65 log lines:\n'
        tail -n 65 "$LOG"
        printf '\nThe Konsole shell remains open. Send the log for a source fix:\n%s\n' "$LOG"
        exit "$result"
    fi
    printf '\nBUILD SUCCEEDED.\n'
fi

if [[ "$MODE" == '--build-only' ]]; then
    printf 'You can launch later using: bash run-r4.sh --run-only\n'
    exit 0
fi
if [[ ! -x "$ROOT/target/release/nesticles" ]]; then
    printf '\nNo executable found at %s/target/release/nesticles\n' "$ROOT"
    printf 'Run: bash run-r4.sh --build-only\n'
    exit 13
fi
printf '\nLaunching Nesticles R4...\n'
if command -v distrobox >/dev/null 2>&1; then
    distrobox enter "$BOX" -- bash -c '
      export PATH="$HOME/.cargo/bin:$PATH"
      export RUST_BACKTRACE=1
      cd "$1" || exit 30
      exec ./target/release/nesticles
    ' bash "$ROOT" > "$RUNLOG" 2>&1
    result=$?
else
    (cd "$ROOT" && RUST_BACKTRACE=1 ./target/release/nesticles) > "$RUNLOG" 2>&1
    result=$?
fi
printf '\nNesticles closed (exit code %s).\n' "$result"
if [[ "$result" -ne 0 ]]; then
    printf 'Runtime log (last 65 lines):\n'
    tail -n 65 "$RUNLOG"
    printf '\nComplete log: %s\n' "$RUNLOG"
fi
exit "$result"
