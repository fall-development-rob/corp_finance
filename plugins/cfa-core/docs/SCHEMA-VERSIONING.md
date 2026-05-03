# cfa-core schema versioning policy

This document defines what constitutes a breaking change to the wire format
between Claude Code (the MCP client) and the cfa-core MCP server. The wire
format is the JSON returned by every tool — its shape is the contract.

## Versions in play

There are three independently-evolving versions and they should not be
confused.

| Version | Source | What it tracks |
|---|---|---|
| **Plugin version** | `plugins/cfa-core/.claude-plugin/plugin.json:version` | The marketplace-visible release. Bumps whenever any of the others bump. |
| **Package version** | `crates/corp-finance-wasm/Cargo.toml:version` | The WASM binary. Returned in every tool result as `metadata.version`. Bumps on any compute change. |
| **Schema version** | `plugins/cfa-core/mcp/src/schemas.json:schema_version` | The input/output schema contract. Bumps **only** on wire-format change. |

A change that adds a new tool but doesn't touch any existing tool's input or
output bumps the package version (new compute) but **not** the schema version
(no existing contract changed).

A change that renames a field on `WaccOutput` bumps the schema version even if
the implementation is otherwise unchanged.

## Schema version is semver

Format: `MAJOR.MINOR.PATCH`. Currently `1.0.0`.

| Change type | Bump | Examples |
|---|---|---|
| **Removing a field** from any input or output struct | MAJOR | Drop `WaccOutput.cost_of_debt_pretax`. |
| **Renaming a field** | MAJOR | `WaccOutput.wacc` → `WaccOutput.weighted_average_cost_of_capital`. |
| **Changing a field's type** | MAJOR | `WaccInput.beta: Decimal` → `WaccInput.beta: f64`. |
| **Making a previously-optional field required** | MAJOR | `WaccInput.size_premium: Option<Rate>` → `WaccInput.size_premium: Rate`. |
| **Adding a required field to an input** | MAJOR (breaks existing callers) | New mandatory `WaccInput.industry: String`. |
| **Removing a tool** entirely | MAJOR | Drop `calculate_wacc`. |
| **Adding an optional field to an input** | MINOR | New optional `WaccInput.sector: Option<String>`. |
| **Adding a field to an output** | MINOR | New `WaccOutput.cost_of_preferred: Decimal`. |
| **Adding a new tool** | MINOR | New `compute_apv` MCP tool. |
| **Renaming a doc comment / improving description** | PATCH | None of the structural shape changed. |
| **Bug fix that doesn't change shape** | PATCH | `calculate_wacc` was returning negative; now returns abs value. Output shape identical. |

## How to bump

When opening a PR that changes any `*Input` or `*Output` struct:

1. Run `python3 scripts/gen-zod-schemas.py` to refresh `schemas.json`.
2. Run `python3 scripts/diff-schemas.py origin/main HEAD` to see what changed.
3. Update `schemas.json:schema_version` per the table above.
4. If MAJOR, also update `crates/corp-finance-wasm/Cargo.toml` MAJOR and the
   plugin manifest. Add a "Migration from N → N+1" section to this file.
5. CI runs `scripts/check-schema-compat.sh` automatically; it'll fail if the
   diff shows a structural change but `schema_version` wasn't bumped.

## Deprecation policy

We never remove a tool or field in a single release. The cycle:

1. **Deprecate**: mark deprecated in the doc comment and `metadata.deprecated`.
   MINOR bump.
2. **Wait at least one MINOR release** before removing.
3. **Remove**: MAJOR bump. Removed tools become 410 Gone responses for one
   more MINOR release before disappearing entirely.

Three concurrent MINOR versions of overlap is the target. Faster than that
breaks downstream skills/commands; slower than that bloats the surface.

## Migration history

### 1.0.0 (initial)

- 244 tools shipped via `wasm_tool!` macro (T1.1)
- Per-tool zod schemas with field-level hints (T1.2)
- 8 hand-written rich examples + 235 auto-generated shape examples (T1.3)
