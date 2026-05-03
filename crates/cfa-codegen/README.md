# cfa-codegen

Durable replacement for the 6 Python codegen scripts under `scripts/gen-*.py` and `scripts/diff-schemas.py` / `scripts/link-skills-tools.py`. One Rust binary, one parser per source format, one emitter per output artifact.

## Why

The Python scripts use regex parsing of Rust source. This is fragile in practice: we hit several bugs already (`Vec<(String, Decimal)>` getting truncated at the comma, doc comments with embedded newlines breaking TS string literals, structs with the same name in different modules silently picking the wrong one based on filesystem walk order). `cfa-codegen` switches to `syn` AST parsing for the corp-finance-core source walk and consolidates all six scripts behind one CLI.

## Architecture

```
src/
├── main.rs                    # clap dispatcher, one subcommand per Python script
├── lib.rs                     # re-exports the public modules
├── parsers/
│   ├── napi.rs                # regex parse of packages/bindings/src/lib.rs
│   └── rust_structs.rs        # syn AST walk of crates/corp-finance-core/src/
├── emitters/
│   ├── wasm.rs                # → crates/corp-finance-wasm/src/lib.rs
│   ├── mcp_tools.rs           # → plugins/cfa-core/mcp/src/tools.ts
│   ├── zod.rs                 # → plugins/cfa-core/mcp/src/{schemas.ts,schemas.json}
│   └── examples.rs            # → plugins/cfa-core/examples/<tool>.json
├── commands/
│   ├── diff_schemas.rs        # `cfa-codegen diff-schemas <a> <b>`
│   └── lint_skills.rs         # `cfa-codegen lint-skills [--check]`
└── data/
    ├── descriptions.rs        # DESCRIPTION_OVERRIDES / SKIP / LEGACY_ALIASES (gen-mcp-tools.py)
    ├── examples.rs            # RICH_EXAMPLES (gen-examples.py)
    ├── runtime_baselines.rs   # RUNTIME_BASELINES (gen-mcp-tools.py)
    └── ext_prefixes.rs        # EXTERNAL_TOOL_PREFIXES / KNOWN_NON_TOOLS (link-skills-tools.py)
```

The NAPI parser stays regex-based — the file is mechanically generated and the parse is one regex per binding. The Rust struct walk uses `syn` 2.x because regex was the source of every bug we hit.

## CLI

```text
cfa-codegen wasm-bindings       # writes crates/corp-finance-wasm/src/lib.rs
cfa-codegen mcp-tools           # writes plugins/cfa-core/mcp/src/tools.ts
cfa-codegen zod-schemas         # writes plugins/cfa-core/mcp/src/{schemas.ts,schemas.json}
cfa-codegen examples            # writes plugins/cfa-core/examples/*.json
cfa-codegen diff-schemas <a> <b>  # diffs two schemas.json snapshots
cfa-codegen lint-skills [--check] # validates skill/command tool refs
cfa-codegen all                 # runs the four emitters in order
```

`--repo <path>` overrides workspace detection (otherwise walks up from CWD looking for a `[workspace]` Cargo.toml).

## Migration path

The 6 Python scripts are kept in `scripts/` for one release, with a `DEPRECATED` header. Removal is planned for v0.3.

| Python script | Replacement |
|---|---|
| `gen-wasm-bindings.py` | `cfa-codegen wasm-bindings` |
| `gen-mcp-tools.py` | `cfa-codegen mcp-tools` |
| `gen-zod-schemas.py` | `cfa-codegen zod-schemas` |
| `gen-examples.py` | `cfa-codegen examples` |
| `diff-schemas.py` | `cfa-codegen diff-schemas` |
| `link-skills-tools.py` | `cfa-codegen lint-skills` |

CI and developer scripts can drop the Python invocation and call `cfa-codegen all` (≈80ms cold, ≈20ms warm) for a complete regen.

## Parity with the Python scripts

`scripts/verify-codegen-parity.sh` runs all four Python emitters and the Rust `all` subcommand, then `diff -u` each output pair. The script exits non-zero only on **unexpected** divergences; the documented divergences below are tolerated by name.

Byte-identical:

- `crates/corp-finance-wasm/src/lib.rs`
- `plugins/cfa-core/mcp/src/tools.ts`
- `plugins/cfa-core/mcp/src/schemas.json`
- `plugins/cfa-core/examples/README.md`

Deliberate divergences (Rust output is more correct — 4 schemas, 4 examples):

- `plugins/cfa-core/mcp/src/schemas.ts` — when two structs share a name across modules (e.g. `WaterfallInput` exists in both `pe::waterfall` and `clo_analytics::waterfall`), the Python parser picks one by filesystem walk order, which can disagree with the NAPI binding's actual `input_type` path. The Rust parser uses the fully-qualified module path from the NAPI binding for an exact match. Affects 4 tools in the current source: `analyze_breakeven`, `calculate_waterfall`, `comp_reconciliation`, `run_black_litterman`.
- `plugins/cfa-core/examples/{analyze_breakeven,calculate_waterfall,comp_reconciliation,run_black_litterman}.json` — same root cause, same fix.

These divergences are bug fixes, not formatting changes. They reflect the actual Input struct each NAPI binding points to. The corresponding Python output was emitting a *different* struct's field set under the right tool name — schemas that would not have validated against the real Input struct.

Other duplicate-named struct pairs in the source (where Python and Rust happen to agree on the same module despite Python's name-only lookup): `ReconciliationInput` for `reconcile_accounting`, `BlackLittermanInput` and a few others where the right one was first in rglob order on this filesystem. Once the Python scripts are removed, these cases go away too.

`lint-skills` produces identical aggregate counts (673 references / 78 resolved / 69 external / 526 unresolved on the current source) but the fuzzy "did you mean" suggestions differ: Rust uses a Levenshtein-based similarity ratio while Python's `difflib.get_close_matches` uses `SequenceMatcher.ratio()` (matching-block-based). Both pick reasonable candidates; the rankings differ on borderline edits.

## Build & test

```bash
cargo build -p cfa-codegen        # ~2s warm
cargo test  -p cfa-codegen        # 25 unit tests
cargo run   -p cfa-codegen -- all # full regen ~80ms
```
