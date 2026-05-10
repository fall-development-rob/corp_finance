#!/usr/bin/env bash
# Phase 32 Wave 1 — fail if any workspace member carries a package-lock.json.
set -euo pipefail

violations=()
while IFS= read -r -d '' f; do
  # Exclude node_modules vendoring of lockfiles (e.g., devDeps internals)
  case "$f" in
    */node_modules/*) continue ;;
  esac
  violations+=("$f")
done < <(find packages -mindepth 2 -maxdepth 3 -name package-lock.json -print0 2>/dev/null)

if [ ${#violations[@]} -gt 0 ]; then
  echo "::error::Workspace members may not carry package-lock.json. Found:"
  printf '  - %s\n' "${violations[@]}" >&2
  echo
  for f in "${violations[@]}"; do
    pkg=$(basename "$(dirname "$f")")
    echo "Workspace member '${pkg}' carries a package-lock.json. Workspace lockfiles must live ONLY at the repo root. Delete '${f}' and run \`npm install\` from the root." >&2
  done
  exit 1
fi
echo "[ok] No workspace-local package-lock.json files found."
