# cfa-codegen

Single source of truth for cfa-core's generated artifacts: WASM bindings, MCP tool registry, zod input schemas, per-tool examples, schema diffs, and skill-reference linting. One Rust binary, one parser per source format, one emitter per output artifact.

## Why

The previous regex-based Python scripts hit several real bugs (`Vec<(String, Decimal)>` getting truncated at the comma, doc comments with embedded newlines breaking TS string literals, structs with the same name in different modules silently picking the wrong one based on filesystem walk order). `cfa-codegen` uses `syn` AST parsing for the corp-finance-core source walk and consolidates every codegen step behind one CLI.

## Architecture

```
src/
├── main.rs                    # clap dispatcher, one subcommand per artifact
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
    ├── descriptions.rs        # DESCRIPTION_OVERRIDES / SKIP / LEGACY_ALIASES
    ├── examples.rs            # RICH_EXAMPLES (hand-written tool inputs)
    ├── runtime_baselines.rs   # RUNTIME_BASELINES (per-tool latency hints)
    └── ext_prefixes.rs        # EXTERNAL_TOOL_PREFIXES / KNOWN_NON_TOOLS
```

The NAPI parser stays regex-based — the file is mechanically generated and the parse is one regex per binding. The Rust struct walk uses `syn` 2.x and now also captures enum-shaped Input types (e.g. `MbsAnalyticsInput`) so the zod emitter can produce a discriminated union rather than an opaque `z.any()`.

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

## Self-consistency check

`scripts/verify-codegen-parity.sh` snapshots the four committed generated artifacts, runs `cargo run -p cfa-codegen -- all`, and confirms the regeneration matches what's checked in. It then runs the generator a second time to confirm idempotency. Used by CI to catch drift between Input structs and the published TS schemas / WASM bindings / examples.

## Build & test

```bash
cargo build -p cfa-codegen        # ~2s warm
cargo test  -p cfa-codegen        # unit tests (parsers + emitters)
cargo run   -p cfa-codegen -- all # full regen ~80ms
```
