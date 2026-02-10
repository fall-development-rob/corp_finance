#!/usr/bin/env bash
# version-bump.sh — Bump semantic version across the entire workspace
# Usage: ./scripts/version-bump.sh <major|minor|patch>
#
# This script:
#   1. Reads the current version from the workspace Cargo.toml
#   2. Computes the new version based on the bump type
#   3. Updates all Cargo.toml files (workspace + crates that hardcode version)
#   4. Updates package.json files
#   5. Updates the MCP server version string in index.ts
#   6. Generates a changelog entry from commits since the last tag
#   7. Prepends the new entry to CHANGELOG.md
#   8. Creates a git tag v{new_version}
#   9. Prints a summary

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Validate input ---
BUMP_TYPE="${1:-}"
if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo "Usage: $0 <major|minor|patch>"
    echo ""
    echo "  major  — breaking changes (0.1.0 -> 1.0.0)"
    echo "  minor  — new features    (0.1.0 -> 0.2.0)"
    echo "  patch  — bug fixes       (0.1.0 -> 0.1.1)"
    exit 1
fi

# --- Read current version from workspace Cargo.toml ---
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
CURRENT_VERSION=$(grep -E '^version\s*=\s*"' "$CARGO_TOML" | head -1 | sed -E 's/.*"([0-9]+\.[0-9]+\.[0-9]+)".*/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo "ERROR: Could not read version from $CARGO_TOML"
    exit 1
fi

echo "Current version: $CURRENT_VERSION"

# --- Compute new version ---
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

case "$BUMP_TYPE" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "New version:     $NEW_VERSION"
echo ""

# --- Update workspace Cargo.toml ---
echo "Updating $CARGO_TOML"
sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" "$CARGO_TOML"

# --- Update crate Cargo.toml files that hardcode version (not workspace = true) ---
for CRATE_TOML in "$PROJECT_ROOT"/crates/*/Cargo.toml "$PROJECT_ROOT"/packages/*/Cargo.toml; do
    [ -f "$CRATE_TOML" ] || continue

    # Only update if the file has a hardcoded version (not version.workspace = true)
    if grep -qE '^version\s*=\s*"[0-9]' "$CRATE_TOML"; then
        echo "Updating $CRATE_TOML"
        sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" "$CRATE_TOML"
    fi
done

# --- Update package.json files ---
for PKG_JSON in "$PROJECT_ROOT"/package.json "$PROJECT_ROOT"/packages/*/package.json; do
    [ -f "$PKG_JSON" ] || continue

    if grep -q "\"version\": \"${CURRENT_VERSION}\"" "$PKG_JSON"; then
        echo "Updating $PKG_JSON"
        sed -i "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" "$PKG_JSON"
    fi
done

# --- Update MCP server index.ts version string ---
MCP_INDEX="$PROJECT_ROOT/packages/mcp-server/src/index.ts"
if [ -f "$MCP_INDEX" ]; then
    if grep -q "version: \"${CURRENT_VERSION}\"" "$MCP_INDEX"; then
        echo "Updating $MCP_INDEX"
        sed -i "s/version: \"${CURRENT_VERSION}\"/version: \"${NEW_VERSION}\"/" "$MCP_INDEX"
    fi
fi

# --- Generate changelog entry ---
echo ""
echo "Generating changelog entry..."

CHANGELOG="$PROJECT_ROOT/CHANGELOG.md"
CHANGELOG_GEN="$SCRIPT_DIR/changelog-gen.sh"
TODAY=$(date +%Y-%m-%d)

if [ -x "$CHANGELOG_GEN" ]; then
    NEW_ENTRY=$("$CHANGELOG_GEN" "" "HEAD" "$NEW_VERSION" "$TODAY" 2>/dev/null || true)
else
    echo "WARNING: changelog-gen.sh not found or not executable, skipping changelog generation"
    NEW_ENTRY=""
fi

if [ -n "$NEW_ENTRY" ] && [ -f "$CHANGELOG" ]; then
    echo "Prepending new entry to $CHANGELOG"

    # Create a temp file with the new content
    TMPFILE=$(mktemp)

    # Write header lines (first 6 lines: title, blank, description, blank, semver note, blank)
    head -6 "$CHANGELOG" > "$TMPFILE"

    # Append new entry
    echo "" >> "$TMPFILE"
    echo "$NEW_ENTRY" >> "$TMPFILE"

    # Append the rest of the changelog (skip header, keep old entries)
    tail -n +7 "$CHANGELOG" >> "$TMPFILE"

    mv "$TMPFILE" "$CHANGELOG"
elif [ -n "$NEW_ENTRY" ]; then
    echo "Creating $CHANGELOG"
    {
        echo "# Changelog"
        echo ""
        echo "All notable changes to this project will be documented in this file."
        echo ""
        echo "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),"
        echo "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)."
        echo ""
        echo "$NEW_ENTRY"
    } > "$CHANGELOG"
fi

# --- Create git tag ---
echo ""
echo "Creating git tag: v${NEW_VERSION}"
git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}"

# --- Summary ---
echo ""
echo "============================================"
echo "  Version bump complete: $CURRENT_VERSION -> $NEW_VERSION"
echo "============================================"
echo ""
echo "Updated files:"
git diff --name-only 2>/dev/null || true
echo ""
echo "Tag created: v${NEW_VERSION}"
echo ""
echo "Next steps:"
echo "  1. Review changes:  git diff"
echo "  2. Stage and commit: git add -A && git commit -m \"chore: release v${NEW_VERSION}\""
echo "  3. Push with tags:   git push && git push --tags"
