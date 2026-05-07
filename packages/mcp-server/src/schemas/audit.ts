import { z } from "zod";

/**
 * Audit bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/audit/surface_audit.rs::SurfaceManifest`.
 * Per ADR-017 §1 / RUF-AUD-001..003.
 */

const SurfaceEnum = z.enum(["cli", "mcp", "skill", "plugin"]);

export const SurfaceAuditComputeSchema = z.object({
  manifest: z
    .object({
      surface: SurfaceEnum.describe(
        "Which CFA surface produced this event."
      ),
      surface_event_id: z
        .string()
        .describe(
          "CLI subcommand name / MCP tool name / slash-command id / plugin hook id."
        ),
      command_args: z
        .record(z.unknown())
        .describe(
          "Canonical JSON of the command arguments / tool input. Keys are sorted before hashing."
        ),
      output_paths: z
        .array(z.string())
        .describe(
          "Output file paths produced by the event. Sorted lex inside the hasher."
        ),
    })
    .describe("SurfaceManifest input to the djb2 hasher."),
});
