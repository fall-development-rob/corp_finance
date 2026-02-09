#!/bin/sh
# Install git hooks from scripts/hooks/ into .git/hooks/
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_SRC="$REPO_ROOT/scripts/hooks"
HOOKS_DST="$REPO_ROOT/.git/hooks"

if [ ! -d "$HOOKS_SRC" ]; then
    echo "ERROR: $HOOKS_SRC not found"
    exit 1
fi

for hook in "$HOOKS_SRC"/*; do
    hook_name="$(basename "$hook")"
    cp "$hook" "$HOOKS_DST/$hook_name"
    chmod +x "$HOOKS_DST/$hook_name"
    echo "Installed $hook_name"
done

echo "All hooks installed."
