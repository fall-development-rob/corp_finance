import { z } from "zod";

/**
 * Memory bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/memory/types.rs`:
 *   - RunSummary aggregate (HNSW + BM25 indexed)
 *   - MemoryQuery (hybrid retriever input)
 *   - CfaSession persistence (.cfa-session archive)
 *
 * Per ADR-016 / RUF-MEM-001..009.
 */

const SurfaceEnum = z.enum(["cli", "mcp", "skill", "plugin"]);

const RunSummarySchema = z.object({
  run_id: z
    .string()
    .uuid()
    .optional()
    .describe("UUID v7 invocation id. If omitted the core allocates one."),
  surface: SurfaceEnum.describe("Runtime surface that produced this record."),
  surface_event_id: z
    .string()
    .describe(
      "CLI subcommand path / MCP tool name / slash command id / hook id."
    ),
  surface_audit_hash: z
    .string()
    .describe("djb2:0x<8-hex> hash over the surface manifest (RUF-AUD-INV-002)."),
  summary_text: z
    .string()
    .describe("Free-form summary text indexed by BM25 and embedded for HNSW."),
  embedding: z
    .array(z.number())
    .describe("Embedding vector, length must equal HnswMemoryIndex dim."),
  ts: z
    .string()
    .optional()
    .describe("RFC3339 timestamp. Defaults to now() when omitted."),
  tenant_id: z
    .string()
    .optional()
    .describe("Optional tenant scope for multi-tenant deployments."),
});

export const SurfaceMemoryIngestSchema = z.object({
  run_summary: RunSummarySchema.describe(
    "Canonical RunSummary record to index."
  ),
});

export const SurfaceMemoryFindSchema = z.object({
  query: z
    .object({
      embedding: z
        .array(z.number())
        .optional()
        .describe("Vector query for the HNSW index."),
      text: z
        .string()
        .optional()
        .describe("Free-form text query for the BM25 index."),
      surface: SurfaceEnum.optional().describe(
        "Optional surface filter (AND-combined)."
      ),
      limit: z.number().int().positive().describe("Max results to return."),
      tenant_id: z
        .string()
        .optional()
        .describe("Optional tenant scope (AND-combined)."),
    })
    .describe(
      "Hybrid retriever query. At least one of embedding or text required."
    ),
});

export const SurfaceSessionSaveSchema = z.object({
  session: z
    .record(z.unknown())
    .describe(
      "CfaSession object: { session_id, surface, run_summaries, created_at, updated_at }."
    ),
  path: z
    .string()
    .describe("Absolute filesystem path to write the .cfa-session archive."),
});

export const SurfaceSessionRestoreSchema = z.object({
  path: z
    .string()
    .describe("Absolute filesystem path to a .cfa-session archive."),
});
