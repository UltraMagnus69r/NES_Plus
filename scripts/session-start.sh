#!/bin/sh
# Start a Nesticles session on the Fedora laptop or the Bazzite desktop.
# This fetches and reports. It does not reset, merge, or discard local work.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

echo "whoami: $(whoami)"
echo "hostname: $(hostname)"
echo "pwd: $(pwd)"
if [ -f /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    echo "os: ${PRETTY_NAME:-unknown}"
fi

case "$(pwd)" in
    /agent/repos/NES_Plus|/agent/repos/NES_Plus/*)
        echo "STOP: this is a cloud agent checkout. Nesticles is developed on the local Fedora or Bazzite machine." >&2
        exit 2
        ;;
esac
if [ "$(whoami)" = "ubuntu" ] && printf '%s' "$(pwd)" | grep -q '/agent/repos/'; then
    echo "STOP: ubuntu user inside a cloud agent path. Do not continue." >&2
    exit 2
fi

TOP=$(git rev-parse --show-toplevel)
ORIGIN=$(git remote get-url origin)
echo "repository: $TOP"
echo "origin: $ORIGIN"
case "$ORIGIN" in
    *UltraMagnus69r/NES_Plus*|*ultramagnus69r/nes_plus*) ;;
    *)
        echo "STOP: origin is not the official Nesticles repository." >&2
        exit 2
        ;;
esac

echo
echo "branch: $(git branch --show-current)"
echo "HEAD: $(git rev-parse HEAD)"
echo "status:"
git status --short

echo
echo "fetching origin"
git fetch origin

BRANCH=$(git branch --show-current)
if [ -z "$BRANCH" ]; then
    echo "STOP: detached HEAD. Check out the development branch before editing." >&2
    exit 2
fi
if git rev-parse --abbrev-ref --symbolic-full-name '@{u}' >/dev/null 2>&1; then
    UPSTREAM=$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}')
    AHEAD=$(git rev-list --count "$UPSTREAM..HEAD")
    BEHIND=$(git rev-list --count "HEAD..$UPSTREAM")
    echo "upstream: $UPSTREAM ahead $AHEAD behind $BEHIND"
    if [ "$AHEAD" != 0 ] && [ "$BEHIND" != 0 ]; then
        echo "STOP: local and origin have diverged. Do not reset. Reconcile the commits before editing." >&2
        exit 2
    fi
    if [ "$BEHIND" != 0 ]; then
        echo "This checkout is behind origin. Inspect the incoming commits, then update with git pull --ff-only if the working tree is clean."
        echo "git fetch does not change the checked-out files."
    fi
else
    echo "No upstream is set for $BRANCH yet."
fi

mkdir -p .git/hooks
for hook in pre-commit pre-push; do
    cp "$ROOT/.githooks/$hook" ".git/hooks/$hook"
    chmod 755 ".git/hooks/$hook"
done
chmod 755 "$ROOT/.githooks/pre-commit" "$ROOT/.githooks/pre-push" \
    "$ROOT/scripts/session-start.sh" "$ROOT/scripts/session-finish.sh" \
    "$ROOT/scripts/check-repository-safety.sh"
echo "Local hooks installed from .githooks into .git/hooks."

echo
echo "Read these before editing:"
echo "  AGENTS.md"
echo "  docs/policies/INTELLECTUAL_PROPERTY.md"
echo "  docs/handoffs/CURRENT_PROJECT_STATE.md"
echo "  docs/handoffs/DECISION_LOG.md"
echo "  docs/handoffs/MULTI_MACHINE_WORKFLOW.md"
LATEST=$(ls -1 docs/handoffs/daily/*.md 2>/dev/null | sort | tail -1 || true)
if [ -n "${LATEST}" ]; then
    echo "  $LATEST"
fi
echo
echo "Active product milestone is recorded in docs/handoffs/CURRENT_PROJECT_STATE.md."
echo "Do not start from main unless the owner says so."
