# ADR-023: Office OOXML Serialization — .pptx Write Surface (Phase 29 Wave 8)

## Status: Accepted

## Date: 2026-05-08

## Deciders

- CFA Agent platform engineering
- Corp-finance-core module owner
- MCP tool surface owner
- Compliance / audit owner

## Tags

office, ooxml, pptx, rust, feature-flag, napi, mcp, institutional-documents

## Context

Phase 29 Wave 8 extends the office module from .docx (Wave 7, ADR-022) to .pptx for institutional deck deliverables. Pitch decks, IC committee presentations, sector overviews, and research conference materials are currently produced as markdown with the `workflow-pptx-author` skill; recipients must reformat in PowerPoint before distribution. Institutional counterparties — LPs, boards, underwriters, conference hosts — consistently require native .pptx for branding templates, slide-by-slide comments, and compliance archival.

Three architectural constraints from Waves 6 and 7 carry forward unchanged:

1. **Headless CLI** — the platform runs in CI, managed-agent, and operator-shell contexts with no GUI available. The writer must be a pure Rust library with no windowing, COM, or LibreOffice dependency.
2. **Terminal-deliverable invariant** — the platform has no binary reader surface. .pptx files are terminal deliverables produced by one surface event and consumed by a human or external system. There is no round-trip path back into the agent stack. `WriteDeckResult` (`output_path`, `bytes_written`, `sha256`, `slide_count`) is the only system-of-record handle after write.
3. **Opt-in feature** — the `office` Cargo feature introduced in Wave 6 is the correct gating point. No new feature flag is warranted; the pptx engine (whether a crate or hand-rolled) is an optional dependency under the existing `office` feature alongside `rust_xlsxwriter` and `docx-rs`.

pptx OOXML is significantly more complex than xlsx or docx (multiple relationship files, slide layout inheritance, master slide XML, content-type manifests), which motivates a narrow v1 surface. The parallel agent implementing `crates/corp-finance-core/src/office/pptx.rs` makes the engine selection call (see Options Considered). The ADR records both paths and flags the choice as TBD-by-implementer at the time of authoring.

## Decision

### Engine and Feature Flag

**TBD-by-implementer**: The parallel agent writing `crates/corp-finance-core/src/office/pptx.rs` selects one of:

- An existing pure-Rust pptx crate, if a sufficiently mature one exists (comparable maintenance signal to `docx-rs`: active releases, no unsafe FFI, deterministic output); or
- A hand-rolled OOXML approach using the `zip` and `quick-xml` crates already present in the workspace, with minimal fixed XML templates covering the four slide kinds.

Either path is gated under the existing `office` Cargo feature in `crates/corp-finance-core/Cargo.toml` as an optional dependency. Without the feature, the platform binary is unchanged in size and behavior. No new Cargo feature is introduced.

### Public Rust Surface

Under feature `office`, `corp_finance_core::office` exposes:

- `SlideDeckSpec`, `Slide` (tagged enum: `Title` | `Section` | `Content` | `Table`)
- `WriteDeckResult { output_path, bytes_written, sha256, slide_count }`
- `write_slide_deck(spec: &SlideDeckSpec, out: &Path) -> Result<WriteDeckResult>`
- `write_slide_deck_from_json(input_json: &str) -> Result<String>` (JSON envelope in, JSON out)

`Slide` is a serde-tagged enum; the discriminant key `"kind"` is used for JSON wire form (e.g., `{ "kind": "title", "title": "..." }`) with snake_case payloads. This is consistent with the `"type"` discriminant used by `DocBlock` in Wave 7; the `"kind"` key is chosen to align with the PPTX domain vocabulary and does not affect the structural pattern.

### v1 Scope

Four slide kinds are supported in Wave 8: Title (deck title + optional subtitle), Section (divider slide with heading), Content (heading + bullet list), and Table (heading + data table with rows and columns). The following are explicitly out of v1 scope and deferred to later waves: images, embedded charts, slide transitions, animations, speaker notes, custom themes, custom fonts, master slide customization, hyperlinks, and slide size overrides.

### Terminal-Deliverable Invariant

No MCP tool, NAPI binding, or CLI subcommand accepts a `.pptx` file as input. The office surface is write-only. `WriteDeckResult` is the sole system-of-record handle. This invariant is enforced structurally: `write_slide_deck` takes a `&Path` for output but no MCP tool or NAPI schema defines a field that reads an existing `.pptx` path.

### NAPI Binding

`packages/bindings/src/lib.rs` exports `writeSlideDeck(inputJson: string)` accepting a 1-argument JSON envelope `{ spec: SlideDeckSpec, output_path: string }` and returning `JSON<WriteDeckResult>`. The single-string-argument envelope follows the established NAPI boundary pattern across all corp-finance-core bindings (all inputs arrive as a single JSON string; all outputs are returned as a single JSON string). The binding is excluded from the compiled artifact when the `office` feature is not present.

### MCP Tool

`packages/mcp-server/src/tools/office.ts` registers the MCP tool `office_pptx_write`. Its input schema accepts `spec` (SlideDeckSpec JSON) and `output_path` (string path). It does not define any parameter that accepts an existing `.pptx` as input. The tool is write-only and terminal.

### sha256 Stability

The selected pptx engine must produce deterministic output given the same `SlideDeckSpec`. The `sha256` field of `WriteDeckResult` therefore constitutes a stable audit primitive, consistent with the `WriteWorkbookResult.sha256` and `WriteDocResult.sha256` patterns established in Waves 6 and 7. CI asserts this via the unit test `write_deck_sha256_stability` in `office::pptx::tests`.

### Slide Validation

An empty `slides` Vec is rejected with `InvalidInput` before any pptx-build call. A Title slide with an empty title string is rejected with `InvalidInput`. A Table slide with zero columns is rejected with `InvalidInput`. All validation occurs in `write_slide_deck` before any bytes are written.

### Zip Integrity

Every output file starts with the OOXML/ZIP magic bytes `PK\x03\x04`. The test `write_deck_zip_magic_bytes` asserts this post-write.

## Consequences

### Positive

- Institutional deck deliverables (.pptx) are producible from the headless CLI, NAPI, and MCP surfaces without any dependency on PowerPoint, LibreOffice, or COM automation.
- Pitch-deck, IC committee presentation, and sector-overview templates are unblocked for a future wave; the `SlideDeckSpec` wire form provides a stable schema to build against.
- `sha256` in `WriteDeckResult` is a stable audit primitive reusing the Phase 26 audit pipeline pattern (ADR-017) without additional instrumentation.
- Zip-magic and sha256 checks are identical in structure to Waves 6 and 7 (audit-friendly; the same test harness patterns apply).
- No new Cargo feature flag is required; the existing `office` feature remains the single gating boundary for all binary office deliverables.

### Negative

- pptx OOXML is significantly more complex than xlsx or docx; the v1 surface is intentionally narrow as a result. Images, charts, transitions, animations, speaker notes, custom themes, and master slide customization are deferred.
- If the hand-rolled path is chosen, the writer carries fixed XML templates for each slide kind that will need maintenance as the spec evolves (new slide kinds, layout changes).
- If a pptx crate is chosen and is pre-1.0, major-version bumps require a dedicated PR that regenerates the test golden-set, as with `docx-rs` in Wave 7.
- The `"kind"` discriminant key (vs `"type"` in Wave 7's `DocBlock`) is a minor wire-form inconsistency within the office module; future waves may consider harmonizing, but renaming now would be a wire break.

### Neutral

- The `sha2` dep added in Wave 6 covers sha256 for `WriteDeckResult` as well; no additional hashing dependency is required.
- The pptx engine and `docx-rs` share no code; the `office` feature remains a clean namespace boundary.
- The `zip` and `quick-xml` crates are available in the workspace regardless of which engine path the implementer selects.

## Options Considered

### Option 1: Existing pptx crate (TBD-by-implementer — preferred if mature)

If a pure-Rust pptx crate with active maintenance, no unsafe FFI, and deterministic output exists at implementation time, it is preferred for the same reasons `docx-rs` was preferred over hand-rolled docx in Wave 7: lower boilerplate, better test coverage of edge cases, and reduced risk of malformed ZIP archives. The parallel agent evaluates available crates (e.g., `pptx-rs`, `presentrs`) at implementation time.

### Option 2: Hand-rolled OOXML using zip + quick-xml (fallback — acceptable)

If no mature pptx crate exists, the writer hand-rolls OOXML using `zip` and `quick-xml`. This requires implementing relationship files, content-type manifests, slide layout references, and slide body XML for each of the four slide kinds. Maintenance cost is higher than a crate, but the surface is narrow enough (four slide kinds, no themes, no images) that the template set is tractable. This path is acceptable as a v1 fallback.

### Option 3: Pandoc shell-out (Rejected — not headless, not deterministic)

Pandoc is a powerful document converter but requires a binary installation, produces non-deterministic output (timestamps, metadata), and cannot run in environments without Pandoc on PATH. Incompatible with the headless-CLI and sha256-stability invariants. Rejected.

### Option 4: Markdown to PDF via headless Chromium (Rejected — wrong format)

Headless Chromium can render markdown/HTML to PDF but produces PDF, not .pptx. Does not satisfy the native PowerPoint institutional expectation. Rejected.

### Option 5: Non-OOXML deck format — Reveal.js / Google Slides (Rejected — wrong format)

Reveal.js and Google Slides produce HTML or proprietary formats, not .pptx. Institutional counterparties (LPs, lenders, boards) require native PowerPoint for tracked-changes workflows, branding templates, and compliance archival. Rejected.

## Related Decisions

- ADR-022: Office OOXML Serialization (.docx) — Wave 7 establishes the docx write surface, `WriteDocResult` sha256 pattern, and terminal-deliverable invariant that Wave 8 extends
- ADR-021: Office OOXML Serialization (.xlsx) — Wave 6 establishes the `office` feature, terminal-deliverable invariant, and sha256 audit pattern
- ADR-009: Workflow Auditability — sha256 stability reuses the audit-hash primitive pattern
- ADR-015: Native Orchestration Umbrella — office CLI subcommand fits the four runtime surfaces; the `office` feature gate is consistent with the `federation` feature pattern
- ADR-017: Audit / Cost / Observability — `WriteDeckResult.sha256` is a first-class audit field consumed by the audit pipeline
- ADR-020: Self-Learning Loop — `workflow-pptx-author` / future `workflow-deck-author` can become replayable trajectories when the pptx writer is present

## References

- `crates/corp-finance-core/src/office/` — Rust module (feature `office`)
- `crates/corp-finance-core/Cargo.toml` — pptx engine under `[features] office` as optional dependency
- `packages/bindings/src/lib.rs` — `writeSlideDeck` NAPI export
- `packages/mcp-server/src/tools/office.ts` — `office_pptx_write` MCP tool
- `crates/corp-finance-cli/src/main.rs` — `office pptx write` subcommand (feature-gated)
- OOXML specification: ECMA-376 Part 1 (Office Open XML) — PresentationML
- Specflow contracts: `docs/contracts/feature_office_pptx.yml`
- Supersedes (extension of): ADR-022
