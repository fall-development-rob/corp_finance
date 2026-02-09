#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerValuationTools } from "./tools/valuation.js";
import { registerCreditTools } from "./tools/credit.js";
import { registerPETools } from "./tools/pe.js";
import { registerPortfolioTools } from "./tools/portfolio.js";
import { registerScenarioTools } from "./tools/scenarios.js";
import { registerMATools } from "./tools/ma.js";
import { registerJurisdictionTools } from "./tools/jurisdiction.js";

const server = new McpServer({
  name: "corp-finance-mcp",
  version: "0.1.0",
});

registerValuationTools(server);
registerCreditTools(server);
registerPETools(server);
registerPortfolioTools(server);
registerScenarioTools(server);
registerMATools(server);
registerJurisdictionTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
