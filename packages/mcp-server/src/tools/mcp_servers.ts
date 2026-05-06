import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { mcpServerList } from "../bindings.js";
import { McpServerListSchema } from "../schemas/mcp_servers.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMcpServerTools(server: McpServer) {
  server.tool(
    "mcp_server_list",
    "Enumerate the upstream MCP servers available to a CFA managed agent, grouped by cost tier (free_native / free_public_with_api_key / freemium / paid_vendor). Returns name, tier, description, setup hint, and package_path for each. Lets users discover which MCP servers they can wire into a CFA agent without paying for a vendor subscription. No I/O - registry-only.",
    McpServerListSchema.shape,
    async (params) => {
      const validated = McpServerListSchema.parse(coerceNumbers(params));
      const result = mcpServerList(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
