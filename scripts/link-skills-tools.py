#!/usr/bin/env python3
# DEPRECATED: use crates/cfa-codegen instead. Will be removed in v0.3.
# Equivalent: `cargo run -p cfa-codegen -- lint-skills [--check]`
"""Link cfa-core skills and commands to the tools they reference.

Walks `plugins/cfa-core/skills/*/SKILL.md` and `plugins/cfa-core/commands/cfa/*.md`,
extracts backtick-wrapped tool references from the body, and:

1. Cross-references each reference against the available MCP tool registry
   (cfa-core's own 246 tools plus an allow-list of expected upstream tools
   like `fmp_*` from the cfa-pro companion plugin).
2. Adds a `requires_tools:` array to the frontmatter listing the matched
   cfa-core tools — so the MCP server (or a future skill loader) can verify
   every required tool is connected before the skill runs.
3. Emits a validation report listing references that match no known tool —
   these are the broken commands (e.g. `merton_model` that should be
   `calculate_merton`).

Run with `--check` to fail with non-zero exit if any references are
unresolved (suitable for CI). Run without flags to write updated
frontmatter and print a report.
"""
from __future__ import annotations

import argparse
import difflib
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PLUGIN = REPO / "plugins" / "cfa-core"
SKILLS_DIR = PLUGIN / "skills"
COMMANDS_DIR = PLUGIN / "commands" / "cfa"
SCHEMAS = PLUGIN / "mcp" / "src" / "schemas.json"

# Tools provided by other plugins / MCP servers we expect to be present.
# References to these are not flagged as "unresolved" — they're external
# dependencies, not bugs.
EXTERNAL_TOOL_PREFIXES = (
    "fmp_",  # cfa-pro: Financial Modeling Prep
    "fred_",  # cfa-data: FRED macro
    "edgar_",  # cfa-data: SEC EDGAR
    "yf_",  # cfa-data: Yahoo Finance
    "wb_",  # cfa-data: World Bank
    "figi_",  # cfa-data: OpenFIGI
    "lseg_",  # cfa-pro: LSEG
    "sp_",  # cfa-pro: S&P
    "factset_",  # cfa-pro: FactSet
    "morningstar_",
    "moodys_",
    "pitchbook_",
)

# Aliases preserved by gen-mcp-tools.py — they're also valid references.
LEGACY_ALIASES = {"wacc_calculator", "dcf_model"}

# Tokens that look like tool names but aren't (e.g. variable names from code
# blocks, common nouns, prose).
KNOWN_NON_TOOLS = {
    "input",
    "output",
    "result",
    "metadata",
    "currency",
    "true",
    "false",
    "null",
    "ok",
    "error",
    "warnings",
    "assumptions",
    "methodology",
    "n",
    "i",
    "pub",
    "fn",
    "use",
    "let",
    "mut",
    "wacc",
    "dcf",
    "irr",
    "moic",
    "ebitda",
    "gp",
    "lp",
    "p&l",
}


# ---------------------------------------------------------------------------
# Frontmatter handling
# ---------------------------------------------------------------------------

FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n(.*)$", re.DOTALL)


def parse_frontmatter(text: str) -> tuple[dict, str]:
    """Return (frontmatter_dict, body). If no frontmatter, return ({}, text)."""
    m = FRONTMATTER_RE.match(text)
    if not m:
        return {}, text
    body = m.group(2)
    fm: dict = {}
    cur_key = None
    for line in m.group(1).splitlines():
        if not line.strip():
            continue
        if line.startswith(" ") or line.startswith("\t"):
            # continuation / list item
            if cur_key and isinstance(fm[cur_key], list):
                item = line.strip().lstrip("-").strip()
                if item:
                    fm[cur_key].append(item)
            continue
        if ":" in line:
            k, _, v = line.partition(":")
            k = k.strip()
            v = v.strip().strip('"').strip("'")
            if v == "":
                fm[k] = []
                cur_key = k
            else:
                fm[k] = v
                cur_key = k
    return fm, body


def render_frontmatter(fm: dict, body: str) -> str:
    """Re-emit the file with updated frontmatter, preserving body verbatim."""
    lines = ["---"]
    for k, v in fm.items():
        if isinstance(v, list):
            if not v:
                lines.append(f"{k}: []")
            else:
                lines.append(f"{k}:")
                for item in v:
                    lines.append(f"  - {item}")
        else:
            # Quote if value contains a colon (per YAML)
            if ":" in str(v):
                lines.append(f'{k}: "{v}"')
            else:
                lines.append(f"{k}: {v}")
    lines.append("---")
    return "\n".join(lines) + "\n" + body


# ---------------------------------------------------------------------------
# Tool reference extraction
# ---------------------------------------------------------------------------

# Backtick-wrapped lower_snake_case identifiers. We require at least one
# underscore to filter out single words like `result` or `input`.
TOOL_REF_RE = re.compile(r"`([a-z][a-z0-9_]+)`")


def extract_references(body: str) -> set[str]:
    refs = set()
    for m in TOOL_REF_RE.finditer(body):
        token = m.group(1)
        if token in KNOWN_NON_TOOLS:
            continue
        # Require an underscore (filters out short common nouns) OR the token
        # is a known legacy alias (some aliases like `wacc_calculator` already
        # have one but `dcf_model` is borderline — both pass this filter).
        if "_" not in token:
            continue
        refs.add(token)
    return refs


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def is_external(name: str) -> bool:
    return any(name.startswith(prefix) for prefix in EXTERNAL_TOOL_PREFIXES)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Don't update files; exit non-zero if any unresolved references found.",
    )
    args = parser.parse_args()

    schema_manifest = json.loads(SCHEMAS.read_text())
    known = set(schema_manifest["tools"]) | LEGACY_ALIASES
    print(f"Known cfa-core tools : {len(known)}")

    md_files = list(SKILLS_DIR.rglob("SKILL.md")) + list(COMMANDS_DIR.glob("*.md"))
    print(f"Markdown files       : {len(md_files)}")

    total_refs = 0
    total_resolved = 0
    total_external = 0
    unresolved_per_file: dict[str, list[tuple[str, list[str]]]] = {}
    updated = 0

    for path in sorted(md_files):
        text = path.read_text()
        fm, body = parse_frontmatter(text)
        refs = extract_references(body)
        if not refs:
            continue

        resolved = sorted(r for r in refs if r in known)
        external = sorted(r for r in refs if is_external(r))
        unresolved = sorted(r for r in refs if r not in known and not is_external(r))

        total_refs += len(refs)
        total_resolved += len(resolved)
        total_external += len(external)

        if unresolved:
            with_suggestions = []
            for r in unresolved:
                # Suggest the closest existing tool name as a fix.
                close = difflib.get_close_matches(r, known, n=2, cutoff=0.6)
                with_suggestions.append((r, close))
            unresolved_per_file[str(path.relative_to(REPO))] = with_suggestions

        if not args.check and resolved:
            existing = fm.get("requires_tools", [])
            existing_set = set(existing) if isinstance(existing, list) else set()
            new = sorted(set(resolved) | existing_set)
            if new != list(existing_set) or "requires_tools" not in fm:
                fm["requires_tools"] = new
                if external:
                    fm["requires_external_tools"] = external
                path.write_text(render_frontmatter(fm, body))
                updated += 1

    print(f"References found     : {total_refs}")
    print(f"Resolved (cfa-core)  : {total_resolved}")
    print(f"External (other MCP) : {total_external}")
    print(f"Unresolved           : {sum(len(v) for v in unresolved_per_file.values())}")

    if not args.check:
        print(f"Files updated        : {updated}")

    if unresolved_per_file:
        print("\n## Unresolved references")
        for fpath, items in unresolved_per_file.items():
            print(f"\n  {fpath}")
            for ref, suggestions in items:
                if suggestions:
                    print(f"    - `{ref}` → did you mean: {', '.join(suggestions)}?")
                else:
                    print(f"    - `{ref}` (no close match)")

        if args.check:
            return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
