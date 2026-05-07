import { z } from "zod";

/**
 * Agent invocation audit schemas for Phase 29 Wave 2 (ADR-021).
 *
 * Mirrors `crates/corp-finance-core/src/multi_agent/audit_writer.rs`:
 *   - AgentInvokeRecordInput — flat wire shape for record_invocation_with_audit
 *   - AgentInvokeCompleteInput — flat wire shape for complete_invocation_with_audit
 *   - InvocationAuditPaths — return shape for both operations
 */

export const AgentInvokeEntitySchema = z.object({
  kind: z.string().describe("Entity kind: ticker / issuer / sector / fund / cusip (MAC-INV-004)."),
  value: z.string().describe("Entity value string (e.g. ticker symbol, CUSIP, sector name)."),
});

export const AgentInvokeRecordInputSchema = z.object({
  invocation_id: z
    .string()
    .uuid()
    .describe("UUID v7 identifying this invocation (sortable by time)."),
  target_agent: z
    .string()
    .describe("CFA specialist slug, e.g. cfa-equity-analyst (MAC-INV-002)."),
  input_summary: z
    .string()
    .describe("Short summary of the input handed to the specialist."),
  parent_invocation_id: z
    .string()
    .uuid()
    .nullable()
    .optional()
    .describe("Parent invocation id for sub-calls; null/omitted for root invocations."),
  tenant_id: z
    .string()
    .nullable()
    .optional()
    .describe("Federation tenant scope (MAC-INV-001). Use 'local' for single-tenant installs."),
  manifest_root: z
    .string()
    .min(1)
    .describe("Absolute path to the directory where event and audit files are written."),
});

export const AgentInvokeCompleteInputSchema = z.object({
  invocation_id: z
    .string()
    .uuid()
    .describe("UUID of the invocation being completed."),
  output_summary: z
    .string()
    .describe("Short summary of the specialist output (e.g. 'DCF $182, BUY')."),
  entities: z
    .array(AgentInvokeEntitySchema)
    .default([])
    .describe("Entity references extracted from the specialist output (MAC-INV-004)."),
  tenant_id: z
    .string()
    .nullable()
    .optional()
    .describe("Federation tenant scope (MAC-INV-001)."),
  manifest_root: z
    .string()
    .min(1)
    .describe("Absolute path to the directory where event and audit files are written."),
});

export const InvocationAuditPathsSchema = z.object({
  event_path: z.string().describe("Absolute path to the written event JSON file."),
  audit_path: z.string().describe("Absolute path to the companion .audit.json file."),
  surface_audit_hash: z
    .string()
    .regex(/^djb2:0x[0-9a-f]{8}$/)
    .describe("DJB2 surface audit hash (djb2:0x<8 hex digits>)."),
});
