# ADR-021: Office OOXML Serialization — .xlsx Write Surface (Phase 29 Wave 6)

## Status: Accepted

## Date: 2026-05-07

## Deciders

- CFA Agent platform engineering
- Corp-finance-core module owner
- MCP tool surface owner
- Compliance / audit owner

## Tags

office, ooxml, xlsx, rust, feature-flag, napi, mcp, decimal-precision

## Context

Phase 29 Wave 6 introduces the first institutional binary deliverable in the CFA agent stack: a headless .xlsx writer. Prior workflow skills (`workflow-xlsx-author`, `workflow-pptx-author`) described markdown-tabular and CSV layout conventions because the stack had no binary writer. LP pitch books, IC memos with comps tables, and fund admin reports increasingly demand native .xlsx to be usable without a recipient paste step.

Three architectural constraints must hold:

1. **Headless CLI** — the platform runs in CI, managed-agent, and operator-shell contexts where no GUI is available. Any writer must be a pure Rust/Node.js library with no windowing or COM dependency.
2. **Decimal precision** — the platform rule (`all financial math in rust_decimal::Decimal, never f64`) is load-bearing across ~206 corp-finance MCP tools. A cell-boundary exception for .xlsx is unavoidable because the OOXML format stores numbers as IEEE 754 doubles; this exception must be documented, bounded, and validated.
3. **Single-source-of-truth** — the platform has no binary reader surface. .xlsx files are terminal deliverables produced by one surface event and consumed by a human or an external system. There is no round-trip path back into the agent stack. This preserves the single-source-of-truth invariant: the `WriteWorkbookResult` struct (output_path, bytes_written, sha256, sheet_count) is the only system-of-record handle after write.

Alternatives evaluated: hand-rolled OOXML XML, `calamine` (read-only), `umya-spreadsheet` (heavier dependency tree, weaker maintenance signal as of 2026-Q2), `xlsxwriter-c` FFI (unsafe, non-Rust-native). `rust_xlsxwriter` was selected (see Options Considered).

## Decision

### Engine and Feature Flag

Adopt `rust_xlsxwriter ~0.94` as the .xlsx serialization engine. Gate compilation behind a `office` Cargo feature in `crates/corp-finance-core/Cargo.toml`; the feature depends on `rust_xlsxwriter` and `sha2`. Without the feature, the platform binary is unchanged in size and behavior. The CLI subcommand `corp-finance-cli office xlsx write` is similarly feature-gated. Dev-dep `zip 2` is used for test introspection only.

### Public Rust Surface

Under feature `office`, `corp_finance_core::office` exposes:

- `WorkbookSpec`, `SheetSpec`, `CellValue` (text | number | decimal | bool | datetime | empty), `FormulaCell`, `FrozenPanes`, `DefinedName`, `WorkbookProperties`
- `WriteWorkbookResult { output_path, bytes_written, sha256, sheet_count }`
- `write_workbook(spec: &WorkbookSpec, out: &Path) -> Result<WriteWorkbookResult>`
- `write_workbook_from_json(input_json: &str) -> Result<String>` (JSON envelope in, JSON out)

### Decimal→f64 Cell-Boundary Exception

`CellValue::Decimal(String)` cells are handled as follows: the string is first parsed into `rust_decimal::Decimal` (returns `InvalidInput` on parse failure), then converted to `f64` for OOXML emission. This is the **only** f64 in the office module and is documented as the canonical exception to the no-f64 rule. Precision loss at >15 significant decimal figures is an accepted trade-off; the caller is responsible for rounding before submission if higher precision display is required.

### Terminal-Deliverable Invariant

No MCP tool, NAPI binding, or CLI subcommand accepts a `.xlsx` file as input. The office surface is write-only. `WriteWorkbookResult` is the sole system-of-record handle. This invariant is enforced structurally: `write_workbook` takes a `&Path` for output but no MCP tool or NAPI schema defines a field that reads an existing `.xlsx` path.

### NAPI Binding

`packages/bindings/src/lib.rs` exports `writeXlsxWorkbook(inputJson: string)` accepting a 1-argument JSON envelope `{ spec: WorkbookSpec, output_path: string }` and returning `JSON<WriteWorkbookResult>`. The single-string-argument envelope follows the existing NAPI boundary pattern established across all corp-finance-core bindings (all inputs arrive as a single JSON string; all outputs are returned as a single JSON string). The binding is excluded from the compiled artifact when the `office` feature is not present.

### MCP Tool

`packages/mcp-server/src/tools/office.ts` registers the MCP tool `office_xlsx_write`. Its input schema accepts `spec` (WorkbookSpec JSON) and `output_path` (string path). It does not define any parameter that accepts an existing `.xlsx` as input.

### sha256 Stability

The underlying `rust_xlsxwriter` engine is deterministic: given the same `WorkbookSpec`, the writer produces byte-identical output. The `sha256` field of `WriteWorkbookResult` therefore constitutes a stable audit primitive. CI asserts this via the unit test `write_minimal_workbook_one_sheet_one_row`.

### Sheet Name Validation

Sheet names are validated at write time: maximum 31 characters (OOXML limit), unique within the workbook. An empty `sheets` list is rejected as `InvalidInput` before any write occurs.

### Zip Integrity

Every output file starts with the OOXML/ZIP magic bytes `PK\x03\x04`. The test `zip_magic_bytes_present` asserts this post-write.

## Consequences

### Positive

- Institutional binary deliverables (.xlsx) are producible from the headless CLI, NAPI, and MCP surfaces without any external dependency on Excel, LibreOffice, or COM automation.
- The `workflow-xlsx-author` skill can evolve toward executable contract status: its markdown-tabular conventions now have a backing writer.
- `sha256` in `WriteWorkbookResult` is a stable audit primitive usable by the Phase 26 audit pipeline (ADR-017) without additional instrumentation.
- Deferred pptx/docx modules share no code with this one; the `office` feature is a clean namespace boundary.
- The Cargo feature flag keeps the default binary size and compile time unaffected for operators who do not need the xlsx surface.

### Negative

- `Decimal→f64` conversion is lossy above 15 significant figures. Callers working with high-precision cash flows or derivative notionals must round to 15 digits before submitting a `CellValue::Decimal`. This is documented but cannot be enforced at compile time.
- Charts, sparklines, and conditional-formatting rules are deferred to a later sub-wave; Wave 6 delivers data cells and formulas only.
- `.docx` and `.pptx` writers are separate full modules and are not unlocked by the `office` feature; they require separate ADRs and crate dependencies.
- `rust_xlsxwriter ~0.94` is a pre-1.0 crate. Major-version bumps require a dedicated PR that regenerates the test golden-set.

### Neutral

- The `zip 2` dev-dep is test-only and does not appear in production artifacts.
- The `sha2` dep is already pulled in transitively but is now an explicit `office`-feature dep for clarity.

## Options Considered

### Option 1: Hand-rolled OOXML XML (Rejected)

Writing OOXML XML directly (ZIP + XML files) avoids any new crate dependency but requires implementing shared-string tables, style sheets, number format registries, and worksheet relationship files — approximately 2,000–3,000 lines of boilerplate before any cell value is writable. Maintenance cost is high and the risk of producing malformed ZIP archives (unacceptable to Excel) is non-trivial. Rejected in favor of a maintained crate.

### Option 2: calamine (Rejected — wrong direction)

`calamine` is a read-only parser. It does not produce .xlsx output. Rejected outright.

### Option 3: umya-spreadsheet (Rejected — maintenance signal)

`umya-spreadsheet` supports both read and write and has a richer feature set (charts, images). However, as of 2026-Q2 its maintenance cadence has slowed relative to `rust_xlsxwriter`, and its dependency tree is heavier (~30 transitive crates vs. ~12 for `rust_xlsxwriter`). We do not need read access, charts, or images in Wave 6. Rejected; may be revisited if chart support becomes a priority.

### Option 4: xlsxwriter-c FFI (Rejected — unsafe)

The C `libxlsxwriter` binding (`xlsxwriter`) requires `unsafe` FFI, complicates cross-compilation, and breaks the all-safe-Rust invariant for non-Monte-Carlo code. Rejected.

### Option 5 (chosen): rust_xlsxwriter ~0.94

Pure Rust, no unsafe FFI, active maintenance, deterministic output (enabling sha256 stability), compatible with the no-f64 rule modulo the documented cell-boundary exception. Selected.

## Related Decisions

- ADR-009: Workflow Auditability — sha256 stability reuses the audit-hash primitive pattern
- ADR-015: Native Orchestration Umbrella — office CLI subcommand fits the four runtime surfaces; the `office` feature gate is consistent with the `federation` feature pattern
- ADR-017: Audit / Cost / Observability — `WriteWorkbookResult.sha256` is a first-class audit field consumed by the audit pipeline
- ADR-020: Self-Learning Loop — `workflow-xlsx-author` can become a replayable trajectory when the xlsx writer is present

## References

- `crates/corp-finance-core/src/office/` — Rust module (feature `office`)
- `crates/corp-finance-core/Cargo.toml` — `rust_xlsxwriter = { version = "~0.94", optional = true }` and `sha2 = { ..., optional = true }` under `[features] office`
- `packages/bindings/src/lib.rs` — `writeXlsxWorkbook` NAPI export
- `packages/mcp-server/src/tools/office.ts` — `office_xlsx_write` MCP tool
- `crates/corp-finance-cli/src/main.rs` — `office xlsx write` subcommand (feature-gated)
- `rust_xlsxwriter` crate: https://docs.rs/rust_xlsxwriter
- OOXML specification: ECMA-376 Part 1 (Office Open XML)
- Specflow contracts: `docs/contracts/feature_office.yml`
