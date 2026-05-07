import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { surfaceAuditCompute } from "../bindings.js";
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
