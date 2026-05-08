# ADR-025: Rust-to-TypeScript Schema Auto-Generation (Phase 29 Wave 11)

## Status: Accepted

## Date: 2026-05-08

## Deciders

- CFA Agent platform engineering
- MCP tool surface owner
- Rust core library owner
- TypeScript MCP server owner

## Tags

schema-gen, schemars, zod, typescript, rust, mcp, drift, pilot, office

## Context

The MCP server at `packages/mcp-server/src/schemas/` contains 84 hand-maintained zod schema files that mirror Rust types defined in `crates/corp-finance-core/`. This mirroring relationship is structural — every time a Rust type gains a field, is renamed, or changes its serde representation, the corresponding zod file must be updated by hand. In practice, this synchronisation fails on every feature wave.

During the office domain build-out (Waves 6–9) the drift surfaced four times: template input structs, tagged enums with `#[serde(rename_all = "snake_case")]`, optional fields added to existing types, and a HashMap whose key type changed. Each incident required a manual find-and-fix cycle that blocked dependent tool registration.

The platform's runtime validation posture relies on zod: MCP tool inputs are validated at the boundary by zod schemas before dispatch to Rust via NAPI. Replacing hand-maintenance with mechanical generation preserves runtime validation while eliminating the drift surface.

A two-step pipeline (Rust → JSON Schema → zod) is preferred over direct Rust-to-zod tooling because:

1. JSON Schema is a lingua franca: the same intermediate artefacts can later drive OpenAPI generation, Python client stubs, and documentation pipelines without re-running the Rust build.
2. Established ecosystem: `schemars` (Rust) and `json-schema-to-zod` (npm) are mature, independently maintained, and do not require tight coupling between the two language runtimes.

## Decision

### Pipeline

```
[Rust types + #[derive(JsonSchema)]]
        |  cargo test --features schema_gen
        v
[target/schemas/<domain>/*.json]   (JSON Schema, lingua franca)
        |  json-schema-to-zod (npm)
        v
[packages/mcp-server/src/schemas/generated/<domain>/*.ts]
```

The npm script `schemas:gen:office` in `packages/mcp-server` chains both steps. It is the only sanctioned way to produce files under `schemas/generated/`. Hand-editing generated files is forbidden (SCHEMA-INV-001, SCHEMA-INV-005).

### Cargo Feature Gate

Schema generation is gated behind a new cargo feature `schema_gen`. This feature is NOT included in the `full` feature set. The `schemars` derive and the integration test that asserts round-trip wire compatibility (SCHEMA-INV-002) are compiled only when `--features schema_gen` is passed. The runtime build — and therefore the NAPI bindings shipped to the MCP server — is not affected.

### Pilot Scope

Wave 11 pilots on the office domain only (~25 types across `crates/corp-finance-core/src/office/types.rs` and `templates/`). This scope exercises all the hard cases: nested structs, `Option<T>` fields, `HashMap<K, V>`, and tagged enums (`#[serde(tag = "kind", rename_all = "snake_case")]`).

The hand-maintained `packages/mcp-server/src/schemas/office.ts` remains in place during the pilot. Generated schemas land at `schemas/generated/office/` to keep the migration reversible. The 83 remaining hand-maintained schema files are not touched in this wave (SCHEMA-INV-004).

The decision to migrate remaining domains is deferred to subsequent waves, contingent on the pilot demonstrating that the diff against hand-maintained schemas is manageable and that tagged-enum discriminator output (`z.discriminatedUnion`) is correct at all call sites (SCHEMA-INV-003).

### Alternatives Considered

**ts-rs alone (Rejected)** — `ts-rs` emits TypeScript interfaces, not zod schemas. Interfaces carry no runtime validation; replacing zod schemas with interfaces would eliminate the MCP input boundary check entirely.

**specta (Rejected for v1)** — Less stable in the Rust ecosystem at time of this decision; the macros and type-map system are under active redesign. Revisit if `schemars` proves limiting in future waves.

**typeshare (Rejected)** — Language support is narrower (TypeScript, Swift, Kotlin, Go only), the project has had periods of low maintenance activity, and it emits interfaces rather than validators.

**ts-to-zod over ts-rs output (Rejected)** — Chaining `ts-rs` (Rust → TS interfaces) then `ts-to-zod` (TS interfaces → zod) loses semantic information that `schemars` preserves directly in JSON Schema (validation constraints, discriminator hints, default values). Two conversions accumulate loss; one conversion from JSON Schema is preferable.

## Consequences

### Positive

- Drift is eliminated mechanically. Once a Rust type is annotated with `#[derive(JsonSchema)]` and `schemas:gen:office` is in CI, the zod output updates automatically with every Rust change.
- The JSON Schema intermediates are independently useful: future waves can drive OpenAPI spec generation, Python Pydantic models, and hosted documentation from the same artefacts without re-running Rust.
- Generated files have a canonical single source of truth (the Rust struct). There is no ambiguity about which representation is authoritative.
- The `schema_gen` feature gate keeps zero runtime overhead: production NAPI bindings and the MCP server process load no additional code from this pipeline.

### Negative

- A build step is introduced before TypeScript tool code can reflect a Rust type change: `cargo test --features schema_gen` must run, then `npm run schemas:gen:office`, before the generated zod file is updated. This adds friction to the inner-loop development cycle.
- Tagged-enum and `HashMap` edge cases may require manual adjustment at zod call sites. The `json-schema-to-zod` converter does not always emit `z.discriminatedUnion`; a post-generation grep check (SCHEMA-INV-003) catches regressions but does not auto-fix them.
- The pipeline spans two crates (Rust and npm): debugging a schema mismatch requires understanding both the `schemars` derive output and the `json-schema-to-zod` conversion rules, raising the cognitive overhead for contributors unfamiliar with one side.

### Neutral

- `schemars` becomes a dev-dependency of `corp-finance-core` (feature-gated); it does not appear in the published crate's dependency closure.
- `json-schema-to-zod` is an npm dev-dependency of `mcp-server`; it does not ship in the production bundle.
- The generated files are excluded from prettier and eslint by `.prettierignore` / `.eslintignore` rules; the `// AUTO-GENERATED — DO NOT EDIT BY HAND` header is the machine-readable signal for tooling exclusion.

## Related Decisions

- ADR-021: Office OOXML Serialization — the office domain that is the pilot scope for schema generation
- ADR-022: Office DOCX Serialization — office types whose zod schemas are candidates for Wave 11 generation
- ADR-023: Office PPTX Serialization — same
- ADR-024: MCP Tool-Call Audit Middleware — Wave 10 middleware that wraps all MCP tools; generated schemas feed the same validation boundary the audit middleware observes

## References

- `crates/corp-finance-core/src/office/` — Rust types annotated with `#[derive(JsonSchema)]` (pilot scope)
- `packages/mcp-server/src/schemas/generated/office/` — generated zod output directory
- `packages/mcp-server/src/schemas/office.ts` — hand-maintained schema retained during pilot
- Specflow contracts: `docs/contracts/feature_schema_autogen.yml`
