#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerAgentInfrastructureTools } from "./tools/agent_infrastructure/index.js";
import { withAudit } from "./middleware/audit.js";

const server = withAudit(new McpServer({
  name: "corp-finance-mcp",
  version: "1.0.0",
}));

registerAgentInfrastructureTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
