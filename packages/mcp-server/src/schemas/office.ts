import { z } from "zod";

/**
 * Office / OOXML serialisation schemas (Phase 29 Wave 6).
 *
 * Mirrors `crates/corp-finance-core/src/office/types.rs`. Wave 6 ships .xlsx
 * only; .docx and .pptx defer to later waves.
 *
 * Workbooks emitted by these tools are terminal deliverables — the structured
 * `WriteWorkbookResult` (path / sha256 / sheet_count) is the only thing
 * downstream tools should rely on. Don't read the .xlsx back into the system.
 */

const CellValueSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("text"), value: z.string() }),
  z.object({ kind: z.literal("number"), value: z.number() }),
  z.object({ kind: z.literal("decimal"), value: z.string() }),
  z.object({ kind: z.literal("bool"), value: z.boolean() }),
  z.object({
    kind: z.literal("datetime"),
    value: z.string(),
    excel_format: z.string().optional(),
  }),
  z.object({ kind: z.literal("empty") }),
]);

const FormulaCellSchema = z.object({
  row: z.number().int().nonnegative(),
  col: z.number().int().nonnegative(),
  formula: z.string().min(1),
  cached_result: z.number().optional(),
});

const FrozenPanesSchema = z.object({
  row: z.number().int().nonnegative(),
  col: z.number().int().nonnegative(),
});

// --- Cell format & chart schemas (Task A, Phase 29 Wave 6) ---

const CellFormatSchema = z.object({
  num_format: z.string().optional(),
  bold: z.boolean().default(false),
  italic: z.boolean().default(false),
});

const FormattedCellSchema = z.object({
  row: z.number().int().nonnegative(),
  col: z.number().int().nonnegative(),
  format: CellFormatSchema,
});

export const ChartKindEnum = z.enum(["line", "bar", "column", "pie"]);

const ChartSeriesSchema = z.object({
  name: z.string().min(1),
  categories_range: z.string().min(1),
  values_range: z.string().min(1),
});

const ChartSchema = z.object({
  kind: ChartKindEnum,
  anchor_row: z.number().int().nonnegative(),
  anchor_col: z.number().int().nonnegative(),
  title: z.string().optional(),
  series: z.array(ChartSeriesSchema).min(1),
});

// --- Template-render schemas (Task B4, Phase 29 Wave 6) ---

export const TemplateKindEnum = z.enum(["dcf", "comps", "lbo", "three_statement"]);

export const RenderTemplateInputSchema = z.object({
  kind: TemplateKindEnum,
  result_json: z.string().min(1),
});

const SheetSpecSchema = z.object({
  name: z.string().min(1).max(31),
  headers: z.array(z.string()).default([]),
  rows: z.array(z.array(CellValueSchema)).default([]),
  formulas: z.array(FormulaCellSchema).default([]),
  column_widths: z.array(z.number().nonnegative()).default([]),
  frozen_panes: FrozenPanesSchema.optional(),
  cell_formats: z.array(FormattedCellSchema).default([]),
  charts: z.array(ChartSchema).default([]),
});

const DefinedNameSchema = z.object({
  name: z.string().min(1),
  range: z.string().min(1),
});

const WorkbookPropertiesSchema = z.object({
  title: z.string().optional(),
  author: z.string().optional(),
  company: z.string().optional(),
  subject: z.string().optional(),
});

export const WorkbookSpecSchema = z.object({
  sheets: z.array(SheetSpecSchema).min(1),
  defined_names: z.array(DefinedNameSchema).default([]),
  properties: WorkbookPropertiesSchema.default({}),
});

export const WriteXlsxWorkbookInputSchema = z.object({
  spec: WorkbookSpecSchema,
  output_path: z.string().min(1),
});

export const WriteWorkbookResultSchema = z.object({
  output_path: z.string().min(1),
  bytes_written: z.number().int().nonnegative(),
  sha256: z.string().regex(/^[0-9a-f]{64}$/),
  sheet_count: z.number().int().nonnegative(),
});

// ---------------------------------------------------------------------------
// Phase 29 Wave 7 — Word / .docx schemas
// ---------------------------------------------------------------------------

export const TextRunSchema = z.object({
  text: z.string(),
  bold: z.boolean().default(false),
  italic: z.boolean().default(false),
});

export const DocBlockSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("heading"),
    level: z.number().int().min(1).max(3),
    text: z.string().min(1),
  }),
  z.object({
    kind: z.literal("paragraph"),
    runs: z.array(TextRunSchema),
  }),
  z.object({
    kind: z.literal("table"),
    headers: z.array(z.string()),
    rows: z.array(z.array(z.string())),
  }),
  z.object({
    kind: z.literal("bullet_list"),
    items: z.array(z.string()),
  }),
  z.object({
    kind: z.literal("numbered_list"),
    items: z.array(z.string()),
  }),
  z.object({
    kind: z.literal("page_break"),
  }),
]);

export const DocSectionSchema = z.object({
  blocks: z.array(DocBlockSchema),
});

export const WordDocSpecSchema = z.object({
  sections: z.array(DocSectionSchema).min(1),
  properties: WorkbookPropertiesSchema.default({}),
});

export const WriteWordDocInputSchema = z.object({
  spec: WordDocSpecSchema,
  output_path: z.string().min(1),
});

export const WriteDocResultSchema = z.object({
  output_path: z.string().min(1),
  bytes_written: z.number().int().nonnegative(),
  sha256: z.string().regex(/^[0-9a-f]{64}$/),
  section_count: z.number().int().nonnegative(),
});

// ---------------------------------------------------------------------------
// Phase 29 Wave 8 — PowerPoint / .pptx schemas
// ---------------------------------------------------------------------------

export const SlideSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("title"),
    title: z.string().min(1),
    subtitle: z.string().optional(),
  }),
  z.object({
    kind: z.literal("section"),
    heading: z.string().min(1),
  }),
  z.object({
    kind: z.literal("content"),
    title: z.string().min(1),
    bullets: z.array(z.string()).default([]),
  }),
  z.object({
    kind: z.literal("table"),
    title: z.string().min(1),
    headers: z.array(z.string()).min(1),
    rows: z.array(z.array(z.string())).default([]),
  }),
]);

export const SlideDeckSpecSchema = z.object({
  slides: z.array(SlideSchema).min(1),
  properties: WorkbookPropertiesSchema.default({}),
});

export const WriteSlideDeckInputSchema = z.object({
  spec: SlideDeckSpecSchema,
  output_path: z.string().min(1),
});

export const WriteDeckResultSchema = z.object({
  output_path: z.string().min(1),
  bytes_written: z.number().int().nonnegative(),
  sha256: z.string().regex(/^[0-9a-f]{64}$/),
  slide_count: z.number().int().nonnegative(),
});
