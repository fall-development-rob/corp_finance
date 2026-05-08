import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { surfaceAuditCompute } from "../bindings.js";
// SurfaceAuditComputeSchema: hand-maintained MCP input envelope (manifest wrapper).
// Migration note (Wave 13): SurfaceManifestSchema, AuditManifestSchema, and
// SurfaceSchema from generated/audit are available in
// src/schemas/generated/audit/index.js but are NOT imported here due to gap 1:
// the post-processor mis-rewrites the Surface string-enum as
// z.discriminatedUnion("kind", [z.literal(...)]) which is invalid zod and
// causes TS2740. That gap is pre-existing across memory/observability/audit
// domains and is tracked for resolution in a future wave.
// All three schemas with Rust counterparts are accessible via the generated
// index for external consumers; this handler keeps the hand-maintained schema.
import { SurfaceAuditComputeSchema } from "../schemas/audit.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerAuditTools(server: McpServer) {
  server.tool(
    "surface_audit_compute",
    "Compute the deterministic djb2 surface audit hash over a SurfaceManifest. Input: { surface, surface_event_id, command_args, output_paths }. Output: djb2:0x<8-hex> hash invariant under cosmetic key/path reordering. RUF-AUD-002 / RUF-AUD-INV-002.",
    SurfaceAuditComputeSchema.shape,
    async (params) => {
      const validated = SurfaceAuditComputeSchema.parse(coerceNumbers(params));
      const result = surfaceAuditCompute(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
