/**
 * Phase 29 Wave 11 — Generated zod schemas for the office domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:office` to refresh.
 * Source of truth: crates/corp-finance-core/src/office/types.rs + templates/*.rs
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 * Wave 12a fixes: Gap 1 (inline_subschemas) + Gap 2 (discriminatedUnion rewrite) closed.
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/office.ts) — remaining gaps
 * =============================================================================
 *
 * 1. `sheets` min-length constraint: hand-maintained uses `.min(1)` on the sheets
 *    array; generated emits `z.array(...)` with no minimum. Callers relying on the
 *    min-1 guard must keep it at the call site.
 *
 * 2. Optional vs nullable: generated emits `z.union([z.string(), z.null()]).optional()`
 *    for `Option<String>` fields (both nullable AND optional in JSON Schema); hand-
 *    maintained uses `.optional()` only. Zod parse accepts both; the difference
 *    surfaces only when the caller passes explicit `null` — generated schemas allow
 *    it, hand-maintained schemas reject it.
 *
 * 3. sha256 regex: hand-maintained adds `.regex(/^[0-9a-f]{64}$/)` for format
 *    validation; generated emits a plain `z.string()`. Call sites relying on sha256
 *    format validation must keep using hand-maintained WriteWorkbookResultSchema.
 * =============================================================================
 */

import { z } from "zod";

// Re-export all generated schemas with stable, qualified names.

export { CellValueSchema } from "./CellValue.js";
export { FormulaCellSchema } from "./FormulaCell.js";
export { FrozenPanesSchema } from "./FrozenPanes.js";
export { SheetSpecSchema } from "./SheetSpec.js";
export { DefinedNameSchema } from "./DefinedName.js";
export { WorkbookPropertiesSchema } from "./WorkbookProperties.js";
export { WorkbookSpecSchema } from "./WorkbookSpec.js";
export { WriteWorkbookResultSchema } from "./WriteWorkbookResult.js";
export { CellFormatSchema } from "./CellFormat.js";
export { FormattedCellSchema } from "./FormattedCell.js";
export { ChartKindSchema } from "./ChartKind.js";
export { ChartSeriesSchema } from "./ChartSeries.js";
export { ChartSchema } from "./Chart.js";
export { TextRunSchema } from "./TextRun.js";
export { DocBlockSchema } from "./DocBlock.js";
export { DocSectionSchema } from "./DocSection.js";
export { WordDocSpecSchema } from "./WordDocSpec.js";
export { WriteDocResultSchema } from "./WriteDocResult.js";
export { SlideSchema } from "./Slide.js";
export { SlideDeckSpecSchema } from "./SlideDeckSpec.js";
export { WriteDeckResultSchema } from "./WriteDeckResult.js";
export { TemplateKindSchema } from "./TemplateKind.js";
export { RenderTemplateInputSchema } from "./RenderTemplateInput.js";
export { DocTemplateKindSchema } from "./DocTemplateKind.js";
export { RenderDocTemplateInputSchema } from "./RenderDocTemplateInput.js";
export { DeckTemplateKindSchema } from "./DeckTemplateKind.js";
export { RenderDeckTemplateInputSchema } from "./RenderDeckTemplateInput.js";
export { IcMemoInputSchema } from "./IcMemoInput.js";
export { ResearchInitInputSchema } from "./ResearchInitInput.js";
export { PitchDeckInputSchema } from "./PitchDeckInput.js";
export { IcPresentationInputSchema } from "./IcPresentationInput.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
