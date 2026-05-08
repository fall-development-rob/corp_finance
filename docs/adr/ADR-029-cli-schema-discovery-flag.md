# ADR-029: CLI Schema Discovery Flag (`cfa <tool> --schema`)

## Status: Accepted

## Date: 2026-05-08

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`cli`, `schema`, `schemars`, `dx`, `credit-module`, `feature-gate`, `discoverability`

## Context

During a credit-tools demo session on 2026-05-08, three input-shape mismatches surfaced in quick succession, blocking the demo. All three errors shared the same root cause: the caller — whether a human or a downstream agent — had no programmatic way to discover the expected JSON input shape for a CLI subcommand without reading Rust source.

Post-mortem analysis identified three distinct DX gaps:

1. **Input-shape opacity (this ADR)** — No schema discoverability at the CLI surface. Users must read `crates/corp-finance-cli/src/` and the matching `corp-finance-core` input types to learn what fields are required, what their types are, and which enum variants are valid.
2. **Serde enum variant friction** — Variant names on the wire (`MaxOf`/`MinOf`) differ from the natural business names users type (`Maximum`/`Minimum`). This is a separate naming-alignment problem.
3. **CLI metadata-wrapping** — The CLI wraps tool output in `{result, metadata, ...}`. Downstream tools expecting the raw inner type cannot pipe one tool's output into the next without a stripping step. This is a separate pipeline-composition problem.

This ADR addresses gap (1) only. Gaps (2) and (3) are deferred to future waves.

The schemars infrastructure was introduced as part of the Wave-11+ TypeScript auto-generation pipeline documented in ADR-025. The derive macro exists in the workspace and the pipeline runs successfully. What is missing is a CLI-layer entry point that exposes the schema to an interactive caller without requiring the MCP server or the TS auto-gen pipeline to be running.

## Decision

Add a `--schema` flag to the `corp-finance-cli` binary, gated behind a `cli_schema` Cargo feature, that prints the JSON Schema for a subcommand's input type and exits.

### 1. Cargo feature gate

A new feature `cli_schema` is declared in `crates/corp-finance-cli/Cargo.toml`. It carries:
- `schemars` as an optional dependency (pulled in only when the feature is active).
- `corp-finance-core/schema_gen` as the feature that enables `#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]` annotations on input types.

The default CLI build (`cargo build -p corp-finance-cli`) must compile without `schemars` in the dependency tree and must behave identically to the pre-Wave-17c CLI.

### 2. Top-level `--schema` flag and dispatch function

A `--schema` boolean flag is added at the top level of the CLI argument parser. When set, control passes to `print_schema(subcommand: &str)` before any tool execution. `print_schema` matches on the subcommand name and calls `schemars::schema_for!` against the corresponding input type, printing the result as pretty-printed JSON to stdout, then exits 0.

If the subcommand is not yet wired in `print_schema`, the function prints a structured error message to stderr that names the source file and line where the match arm must be added, together with the `#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]` annotation the input type must carry. The function then exits non-zero. It never silently succeeds for an un-wired subcommand.

### 3. Pilot wiring — 4 credit subcommands

Wave 17c wires the following subcommands as the pilot cohort:

| Subcommand | Input type |
|---|---|
| `credit-metrics` | `CreditMetricsInput` |
| `altman-zscore` | `AltmanZScoreInput` |
| `debt-capacity` | `DebtCapacityInput` |
| `covenant-test` | `CovenantTestInput` |

These four types also receive `#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]` annotations in `crates/corp-finance-core/src/credit/`. Subsequent waves wire the remaining ~76 subcommands module-by-module, following the same pattern used for the Wave-16 WASM port batches.

### 4. Schema reflects serde wire form

The schema is generated from `schemars::schema_for!` against the same type that `serde` serialises. Enum variants appear in the wire-rendered form (e.g., `MaxOf`/`MinOf` for a `#[derive(Serialize)]` type with no rename). This is intentional: the schema is the JSON contract, not the Rust API. Gap (2) — the mismatch between wire form and business names — is tracked separately.

### 5. Wave extension protocol

Subsequent waves extend schema coverage by:
1. Adding `#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]` to the target module's input types.
2. Adding a match arm in `print_schema` pointing to each new input type.
3. Running the CI feature-matrix build to confirm both the default and `cli_schema` builds pass.

## Alternatives Considered

**Hand-maintained schema files** — Rejected. Hand-maintained schemas drift from the implementation immediately. The schemars pipeline guarantees the schema is derived from the same source-of-truth as the runtime behaviour.

**Zod schemas printed by the MCP server** — Rejected. Requires the MCP server to be running, which adds a daemon dependency for a DX tool that should be available in any shell. The MCP server is not always active during development or in CI.

**`--help` extension** — Rejected. Clap's `--help` renders human-readable prose and positional argument descriptions; it does not model nested JSON object schemas. Extending `--help` to carry schema information would require duplicating field documentation by hand.

**`cfa schema <tool>` as a top-level subcommand** — Rejected on discoverability grounds. A flag on the tool itself (`cfa credit-metrics --schema`) is colocated with the tool invocation, so a user who knows the tool name can discover its schema without learning a separate `schema` subcommand. A top-level subcommand requires knowing both the subcommand name and the tool name.

**Auto-scan for schemars-annotated types via a build-time proc macro or build.rs** — Deferred. This would eliminate the manual match-arm maintenance in `print_schema`. It is architecturally attractive but adds build-time complexity before the pattern is stable. Revisit after coverage reaches 30+ subcommands and the match-arm maintenance burden becomes measurable.

## Consequences

### Positive

- Zero ambiguity on input shape: any caller can run `cfa <tool> --schema` and get a machine-readable JSON Schema without reading Rust source.
- Agents and human operators gain a discoverable, stable contract. Downstream tooling (agent orchestrators, CI validators, TypeScript SDK consumers) can validate their payloads against the schema before invoking the tool.
- The schemars output reuses the same source-of-truth as the Wave-11+ TS auto-gen pipeline from ADR-025. There is no second schema to maintain.
- The friendly error for un-wired subcommands turns a silent failure into a guided fix: the error message names the exact file and line and the annotation required.

### Negative

- Feature-gated extension work: every new subcommand requires a match arm in `print_schema` and a schemars annotation on its input type. This is mechanical but not automatic. Until the auto-scan approach is adopted, the match-arm list is a manually maintained registry.
- Compile time grows by approximately 5–10 seconds when `cli_schema` is enabled, because `schemars` and its proc-macro codegen add to the dependency graph.
- Schema discovery does not resolve gap (2) (serde enum variant names do not match business names) or gap (3) (CLI metadata-wrapping breaks piping). Those remain open friction points.
- The pilot covers only 4 of approximately 80 subcommands. The CLI remains partially opaque until subsequent waves extend coverage.

### Neutral

- The `--schema` flag exits before any tool computation runs, so it cannot be used to accidentally trigger a live financial calculation.
- The JSON Schema version emitted is determined by the schemars release in use (currently Draft 7 / 2019-09 compatible). If schemars updates to Draft 2020-12, the output changes but remains valid JSON Schema; no ADR amendment is required.

## Links

- Depends on: ADR-025 (Rust-to-TypeScript Schema Auto-Generation) — the `schema_gen` feature and schemars infrastructure this ADR extends to the CLI surface
- Related: ADR-027 (Wave-16 WASM Port Strategy) — establishes the module-by-module batch extension pattern this ADR follows for progressive schema coverage
- Related: ADR-028 (WASM Bundle Size Optimization) — feature-gate discipline precedent
- `crates/corp-finance-cli/src/main.rs` — `print_schema` dispatch function and `--schema` flag
- `crates/corp-finance-core/src/credit/` — 4 pilot input types with schemars annotations
- `docs/contracts/feature_cli_schema_discovery.yml` — invariants DISCOVER-INV-001..005
