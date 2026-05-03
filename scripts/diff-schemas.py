#!/usr/bin/env python3
# DEPRECATED: use crates/cfa-codegen instead. Will be removed in v0.3.
# Equivalent: `cargo run -p cfa-codegen -- diff-schemas <a> <b>`
"""Diff two `schemas.json` snapshots and classify changes per the cfa-core
schema versioning policy (plugins/cfa-core/docs/SCHEMA-VERSIONING.md).

Usage:
    python3 scripts/diff-schemas.py <ref-a> <ref-b>

Where each ref is either:
    - a git ref (e.g. `origin/main`, `HEAD`, `v1.0.0`) — the script reads
      schemas.json from that ref via `git show`.
    - a path to a schemas.json file on disk.

Output:
    Categorised diff with severity (MAJOR / MINOR / PATCH) so a CI check or a
    code reviewer can decide whether the PR's `schema_version` bump matches
    the actual change.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCHEMAS_PATH = "plugins/cfa-core/mcp/src/schemas.json"


def load(ref: str) -> dict:
    """Load schemas.json from a git ref or filesystem path."""
    p = Path(ref)
    if p.exists():
        return json.loads(p.read_text())
    try:
        out = subprocess.run(
            ["git", "show", f"{ref}:{SCHEMAS_PATH}"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=True,
        )
        return json.loads(out.stdout)
    except subprocess.CalledProcessError as e:
        print(f"error: could not read {SCHEMAS_PATH} from {ref}: {e.stderr}", file=sys.stderr)
        sys.exit(2)


def index_tools(snapshot: dict) -> dict[str, dict]:
    """Return { tool_name: schema_dict } indexed for diffing.

    The current schemas.json doesn't include the per-tool field structure
    (it lists missing entries only). For full diff we'd need to extend
    gen-zod-schemas.py to emit field-level info into the manifest. For now
    we treat each tool as a presence/absence signal plus the missing list.
    """
    tools = {}
    # Anything in the manifest's "missing" list is a passthrough — present
    # but with no typed schema.
    for entry in snapshot.get("missing", []):
        tools[entry["tool"]] = {"typed": False, "struct": entry["struct"]}
    # The manifest doesn't currently list the typed tools by name. Until
    # gen-zod-schemas.py is extended, fall back to inspecting schemas.ts.
    return tools


def classify(a: dict, b: dict) -> dict[str, list[str]]:
    """Return { severity: [reasons] }."""
    out: dict[str, list[str]] = {"MAJOR": [], "MINOR": [], "PATCH": []}

    a_tools = set(a.get("tools", []))
    b_tools = set(b.get("tools", []))

    added = sorted(b_tools - a_tools)
    removed = sorted(a_tools - b_tools)
    if added:
        sample = ", ".join(added[:5]) + (" ..." if len(added) > 5 else "")
        out["MINOR"].append(f"+{len(added)} tools (e.g. {sample})")
    if removed:
        sample = ", ".join(removed[:5]) + (" ..." if len(removed) > 5 else "")
        out["MAJOR"].append(f"−{len(removed)} tools removed (e.g. {sample})")

    a_missing = {entry["tool"] for entry in a.get("missing", [])}
    b_missing = {entry["tool"] for entry in b.get("missing", [])}
    newly_passthrough = sorted(b_missing - a_missing)
    newly_typed = sorted(a_missing - b_missing)
    if newly_passthrough:
        out["MAJOR"].append(
            f"became passthrough: {', '.join(newly_passthrough)} — "
            "regression: a previously-typed schema lost its types"
        )
    if newly_typed:
        out["MINOR"].append(f"became typed: {', '.join(newly_typed)}")

    a_ver = a.get("schema_version", "?")
    b_ver = b.get("schema_version", "?")
    if a_ver != b_ver:
        out["PATCH"].append(f"schema_version field: {a_ver} → {b_ver}")

    return out


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2

    a = load(sys.argv[1])
    b = load(sys.argv[2])

    print(f"Comparing {sys.argv[1]} → {sys.argv[2]}")
    print()

    classified = classify(a, b)

    if not any(classified.values()):
        print("No structural changes detected.")
        return 0

    for severity in ("MAJOR", "MINOR", "PATCH"):
        reasons = classified[severity]
        if not reasons:
            continue
        print(f"## {severity}")
        for r in reasons:
            print(f"  - {r}")
        print()

    if classified["MAJOR"]:
        print("Recommended schema_version bump: MAJOR")
        return 0
    if classified["MINOR"]:
        print("Recommended schema_version bump: MINOR")
        return 0
    print("Recommended schema_version bump: PATCH")
    return 0


if __name__ == "__main__":
    sys.exit(main())
