#!/bin/sh
# Finish a session: safety check, then commit and push only when a message is given.
# Refuses a cloud checkout. Does not force-push, reset, or skip the hooks.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

case "$(pwd)" in
    /agent/repos/NES_Plus|/agent/repos/NES_Plus/*)
        echo "STOP: cloud agent checkout." >&2
        exit 2
        ;;
esac

MESSAGE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --message)
            MESSAGE=${2:-}
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--message \"why this commit exists\"]" >&2
            exit 2
            ;;
    esac
done

for required in \
    AGENTS.md \
    docs/policies/INTELLECTUAL_PROPERTY.md \
    docs/handoffs/CURRENT_PROJECT_STATE.md \
    docs/handoffs/DECISION_LOG.md \
    docs/handoffs/MULTI_MACHINE_WORKFLOW.md
do
    if [ ! -f "$required" ]; then
        echo "STOP: missing $required" >&2
        exit 2
    fi
done

if [ -z "$MESSAGE" ]; then
    echo "No --message given. Checking only. Nothing will be committed or pushed."
    "$ROOT/scripts/check-repository-safety.sh" --staged || true
    git status --short
    exit 0
fi

# Stage modifications to tracked files only. New untracked paths stay out until
# a person looks at them and stages them on purpose.
git add -u
"$ROOT/scripts/check-repository-safety.sh" --staged

if git diff --cached --quiet; then
    echo "STOP: nothing is staged." >&2
    exit 2
fi

git commit -m "$MESSAGE"
BRANCH=$(git branch --show-current)
git push -u origin "$BRANCH"
git fetch origin
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse "origin/$BRANCH")
echo "local:  $LOCAL"
echo "origin: $REMOTE"
if [ "$LOCAL" != "$REMOTE" ]; then
    echo "STOP: origin does not match this commit." >&2
    exit 1
fi
echo "Remote commit verified."
