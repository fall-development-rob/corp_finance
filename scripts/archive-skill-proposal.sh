#!/usr/bin/env bash
# Archive an applied skill remediation proposal to a month-bucketed directory.
#
# Usage: archive-skill-proposal.sh <proposal-file>
#
# Moves <proposal-file> to docs/proposed-skill-updates/archive/YYYY-MM/
# where YYYY-MM is the current month.

set -euo pipefail

PROPOSAL="${1:?usage: archive-skill-proposal.sh <proposal-file>}"

if [[ ! -f "$PROPOSAL" ]]; then
  echo "error: file not found: $PROPOSAL" >&2
  exit 1
fi

ARCHIVE_DIR="docs/proposed-skill-updates/archive/$(date +%Y-%m)"

mkdir -p "$ARCHIVE_DIR"
mv "$PROPOSAL" "$ARCHIVE_DIR/"

echo "Archived $PROPOSAL → $ARCHIVE_DIR/$(basename "$PROPOSAL")"
