#!/bin/sh
# Inspect staged files, the tracked tree, or git history for local-only content.
# Exit 1 when a finding is printed. This does not prove that other content is original.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
if [ "$#" -eq 0 ]; then
    set -- --staged
fi
exec python3 "$ROOT/scripts/repository_safety.py" "$@"
