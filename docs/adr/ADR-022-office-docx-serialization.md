# ADR-022: Office OOXML Serialization — .docx Write Surface (Phase 29 Wave 7)

## Status: Accepted

## Date: 2026-05-07

## Deciders

- CFA Agent platform engineering
- Corp-finance-core module owner
- MCP tool surface owner
- Compliance / audit owner

## Tags

office, ooxml, docx, rust, feature-flag, napi, mcp, institutional-documents

## Context

Phase 29 Wave 7 extends the office module from .xlsx (Wave 6, ADR-021) to .docx for institutional document deliverables. IC memos, research reports, Confidential Information Memoranda (CIMs), and ADR-style documents themselves are currently produced as markdown text; recipients must reformat in Word before distribution. Institutional counterparties — LPs, lenders, board members — consistently require native .docx for tracked-changes workflows, letterhead templates, and compliance archival.

Three architectural constraints from Wave 6 carry forward unchanged:

1. **Headless CLI** — the platform runs in CI, managed-agent, and operator-shell contexts with no GUI available. The writer must be a pure Rust library with no windowing, COM, or LibreOffice dependency.
2. **Terminal-deliverable invariant** — the platform has no binary reader surface. .docx files are terminal deliverables produced by one surface event and consumed by a human or external system. There is no round-trip path back into the agent stack. `WriteDocResult` (`output_path`, `bytes_written`, `sha256`, `section_count`) is the only system-of-record handle after write.
3. **Opt-in feature** — the `office` Cargo feature introduced in Wave 6 is the correct gating point. No new feature flag is warranted; `docx-rs` is an optional dependency under the existing `office` feature alongside `rust_xlsxwriter`.

Alternatives evaluated: hand-rolled OOXML XML, `docx-rust` (older crate), `Pandoc` shell-out. `docx-rs` was selected (see Options Considered).

## Decision

### Engine and Feature Flag

Adopt `docx-rs` as the .docx serialization engine. Gate compilation under the existing `office` Cargo feature in `crates/corp-finance-core/Cargo.toml` as an optional dependency. Without the feature, the platform binary is unchanged in size and behavior. The CLI subcommand `corp-finance-cli office docx write` is similarly feature-gated. No new Cargo feature is introduced.

### Public Rust Surface

Under feature `office`, `corp_finance_core::office` exposes:

- `WordDocSpec`, `DocSection`, `DocBlock` (tagged enum: `Heading` | `Paragraph` | `Table` | `BulletList` | `NumberedList` | `PageBreak`), `TextRun`
- `WriteDocResult { output_path, bytes_written, sha256, section_count }`
- `write_word_doc(spec: &WordDocSpec, out: &Path) -> Result<WriteDocResult>`
- `write_word_doc_from_json(input_json: &str) -> Result<String>` (JSON envelope in, JSON out)

`DocBlock` is a serde-tagged enum; the discriminant key `"type"` is used for JSON wire form (e.g., `{ "type": "Heading", "level": 2, "runs": [...] }`). This matches the serde tagged-enum pattern used across the codebase.

### Terminal-Deliverable Invariant

No MCP tool, NAPI binding, or CLI subcommand accepts a `.docx` file as input. The office surface is write-only. `WriteDocResult` is the sole system-of-record handle. This invariant is enforced structurally: `write_word_doc` takes a `&Path` for output but no MCP tool or NAPI schema defines a field that reads an existing `.docx` path.

### NAPI Binding

`packages/bindings/src/lib.rs` exports `writeWordDoc(inputJson: string)` accepting a 1-argument JSON envelope `{ spec: WordDocSpec, output_path: string }` and returning `JSON<WriteDocResult>`. The single-string-argument envelope follows the established NAPI boundary pattern across all corp-finance-core bindings (all inputs arrive as a single JSON string; all outputs are returned as a single JSON string). The binding is excluded from the compiled artifact when the `office` feature is not present.

### MCP Tool

`packages/mcp-server/src/tools/office.ts` registers the MCP tool `office_docx_write`. Its input schema accepts `spec` (WordDocSpec JSON) and `output_path` (string path). It does not define any parameter that accepts an existing `.docx` as input. The tool is write-only and terminal.

### sha256 Stability

`docx-rs` produces deterministic output given the same `WordDocSpec`. The `sha256` field of `WriteDocResult` therefore constitutes a stable audit primitive, consistent with the `WriteWorkbookResult.sha256` pattern established in Wave 6. CI asserts this via the unit test `write_doc_sha256_stability` in `office::docx::tests`.

### Heading Level Validation

`DocBlock::Heading.level` must be in `1..=3`. Values outside this range (0, 4, and above) are rejected with `InvalidInput` before any `docx_rs` build call.

### Section Validation

An empty `sections` Vec is rejected with `InvalidInput` before any `docx_rs` build call. This mirrors the empty-sheets rejection in Wave 6.

### Zip Integrity

Every output file starts with the OOXML/ZIP magic bytes `PK\x03\x04`. The test `write_doc_zip_magic_bytes` asserts this post-write.

## Consequences

### Positive

- Institutional document deliverables (.docx) are producible from the headless CLI, NAPI, and MCP surfaces without any dependency on Word, LibreOffice, or COM automation.
- IC memo, research report, and CIM templates are unblocked for a future wave; the `WordDocSpec` wire form provides a stable schema to build against.
- `sha256` in `WriteDocResult` is a stable audit primitive reusing the Phase 26 audit pipeline pattern (ADR-017) without additional instrumentation.
- Zip-magic and sha256 checks are identical in structure to Wave 6 (audit-friendly; the same test harness patterns apply).
- No new Cargo feature flag is required; the existing `office` feature remains the single gating boundary for all binary office deliverables.

### Negative

- Images, headers/footers, footnotes, table of contents, and track-changes are deferred (out of v1 scope); this mirrors Wave 6's chart-deferral discipline for .xlsx.
- Per-deliverable document templates (IC memo letterhead, research report cover page, CIM branding) come in a follow-up wave; Wave 7 delivers the structural block surface only.
- `docx-rs` is a pre-1.0 crate. Major-version bumps require a dedicated PR that regenerates the test golden-set.

### Neutral

- The `sha2` dep added in Wave 6 covers sha256 for `WriteDocResult` as well; no additional hashing dependency is required.
- `docx-rs` and `rust_xlsxwriter` share no code; the `office` feature remains a clean namespace boundary between the two writers.

## Options Considered

### Option 1: Hand-rolled OOXML XML (Rejected)

Writing OOXML XML directly (ZIP + XML files) for .docx requires implementing relationship files, content-type manifests, styles, numbering definitions, and document body XML — approximately 2,000–3,000 lines of boilerplate before any paragraph is writable. Maintenance cost is high and the risk of producing malformed ZIP archives is non-trivial. Rejected in favor of a maintained crate.

### Option 2: docx-rust (Rejected — maintenance signal)

`docx-rust` is an older crate with lower download volume and a slower maintenance cadence relative to `docx-rs` as of 2026-Q2. Rejected; may be revisited if `docx-rs` maintenance regresses.

### Option 3: Pandoc shell-out (Rejected — not headless, not deterministic)

Pandoc is a powerful document converter but requires a binary installation, produces non-deterministic output (timestamps, metadata), and cannot run in environments without Pandoc on PATH. Incompatible with the headless-CLI and sha256-stability invariants. Rejected.

### Option 4 (chosen): docx-rs

Pure Rust, no unsafe FFI, active maintenance (~600k downloads/year as of 2026-Q2), deterministic output enabling sha256 stability, compatible with the headless-CLI invariant. Selected.

## Related Decisions

- ADR-021: Office OOXML Serialization (.xlsx) — Wave 6 establishes the `office` feature, terminal-deliverable invariant, and sha256 audit pattern that Wave 7 extends
- ADR-009: Workflow Auditability — sha256 stability reuses the audit-hash primitive pattern
- ADR-015: Native Orchestration Umbrella — office CLI subcommand fits the four runtime surfaces; the `office` feature gate is consistent with the `federation` feature pattern
- ADR-017: Audit / Cost / Observability — `WriteDocResult.sha256` is a first-class audit field consumed by the audit pipeline
- ADR-020: Self-Learning Loop — `workflow-xlsx-author` / future `workflow-docx-author` can become replayable trajectories when their respective writers are present

## References

- `crates/corp-finance-core/src/office/` — Rust module (feature `office`)
- `crates/corp-finance-core/Cargo.toml` — `docx-rs = { version = "...", optional = true }` under `[features] office`
- `packages/bindings/src/lib.rs` — `writeWordDoc` NAPI export
- `packages/mcp-server/src/tools/office.ts` — `office_docx_write` MCP tool
- `crates/corp-finance-cli/src/main.rs` — `office docx write` subcommand (feature-gated)
- `docx-rs` crate: https://docs.rs/docx-rs
- OOXML specification: ECMA-376 Part 1 (Office Open XML)
- Specflow contracts: `docs/contracts/feature_office_docx.yml`
- Supersedes (extension of): ADR-021
