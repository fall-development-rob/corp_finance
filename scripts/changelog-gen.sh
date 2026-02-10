#!/usr/bin/env bash
# changelog-gen.sh — Generate a Keep-a-Changelog entry from conventional commits
# Usage: ./scripts/changelog-gen.sh [FROM_REF] [TO_REF] [VERSION] [DATE]
#   FROM_REF  — git ref to start from (default: last tag, or root if no tags)
#   TO_REF    — git ref to end at (default: HEAD)
#   VERSION   — version label for the header (default: "Unreleased")
#   DATE      — date for the header (default: today)

set -euo pipefail

FROM_REF="${1:-}"
TO_REF="${2:-HEAD}"
VERSION="${3:-Unreleased}"
DATE="${4:-$(date +%Y-%m-%d)}"

# Determine the starting ref if not provided
if [ -z "$FROM_REF" ]; then
    TAG_COUNT=$(git tag -l 'v*' | wc -l)
    if [ "$TAG_COUNT" -ge 2 ]; then
        # Two or more tags: range between last two
        FROM_REF=$(git tag -l 'v*' --sort=-v:refname | sed -n '2p')
    elif [ "$TAG_COUNT" -eq 1 ]; then
        # One tag: from that tag to HEAD
        FROM_REF=$(git tag -l 'v*' --sort=-v:refname | head -1)
    else
        # No tags: from the very first commit
        FROM_REF=$(git rev-list --max-parents=0 HEAD | head -1)
    fi
fi

# Collect commit subjects in the range
if git merge-base --is-ancestor "$FROM_REF" "$TO_REF" 2>/dev/null; then
    COMMITS=$(git log --oneline --format="%s" "${FROM_REF}..${TO_REF}" 2>/dev/null || true)
else
    COMMITS=$(git log --oneline --format="%s" "${FROM_REF}...${TO_REF}" 2>/dev/null || true)
fi

# If FROM_REF is the root commit, include it too
ROOT_COMMIT=$(git rev-list --max-parents=0 HEAD | head -1)
if [ "$FROM_REF" = "$ROOT_COMMIT" ]; then
    ROOT_MSG=$(git log --format="%s" -1 "$ROOT_COMMIT")
    COMMITS=$(printf "%s\n%s" "$COMMITS" "$ROOT_MSG")
fi

if [ -z "$COMMITS" ]; then
    echo "No commits found in range ${FROM_REF}..${TO_REF}" >&2
    exit 0
fi

# Categorize commits
ADDED=""
FIXED=""
CHANGED=""
DEPRECATED=""
REMOVED=""
SECURITY=""

while IFS= read -r line; do
    [ -z "$line" ] && continue

    # Strip the type prefix to get the description
    # Match: type(scope): desc  OR  type: desc
    if echo "$line" | grep -qE '^(feat|fix|build|chore|ci|docs|style|refactor|perf|test|revert)(\(.+\))?(!)?: '; then
        TYPE=$(echo "$line" | sed -E 's/^([a-z]+)(\(.+\))?(!)?: .*/\1/')
        DESC=$(echo "$line" | sed -E 's/^[a-z]+(\(.+\))?(!)?: //')
        SCOPE=$(echo "$line" | sed -E 's/^[a-z]+(\(([^)]+)\))?(!)?: .*/\2/')

        # Prefix description with scope if present
        if [ -n "$SCOPE" ]; then
            DESC="**${SCOPE}**: ${DESC}"
        fi

        case "$TYPE" in
            feat)
                ADDED="${ADDED}- ${DESC}\n"
                ;;
            fix)
                FIXED="${FIXED}- ${DESC}\n"
                ;;
            style|refactor|build|ci|chore|perf|docs|test)
                CHANGED="${CHANGED}- ${DESC}\n"
                ;;
            revert)
                REMOVED="${REMOVED}- ${DESC}\n"
                ;;
        esac
    else
        # Non-conventional commit, put in Changed
        CHANGED="${CHANGED}- ${line}\n"
    fi
done <<< "$COMMITS"

# Output in Keep-a-Changelog format
echo "## [${VERSION}] - ${DATE}"
echo ""

if [ -n "$ADDED" ]; then
    echo "### Added"
    printf "%b" "$ADDED"
    echo ""
fi

if [ -n "$FIXED" ]; then
    echo "### Fixed"
    printf "%b" "$FIXED"
    echo ""
fi

if [ -n "$CHANGED" ]; then
    echo "### Changed"
    printf "%b" "$CHANGED"
    echo ""
fi

if [ -n "$DEPRECATED" ]; then
    echo "### Deprecated"
    printf "%b" "$DEPRECATED"
    echo ""
fi

if [ -n "$REMOVED" ]; then
    echo "### Removed"
    printf "%b" "$REMOVED"
    echo ""
fi

if [ -n "$SECURITY" ]; then
    echo "### Security"
    printf "%b" "$SECURITY"
    echo ""
fi
