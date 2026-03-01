#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerCompanyTools } from './tools/company.js';
import { registerFinancialTools } from './tools/financials.js';
import { registerResearchTools } from './tools/research.js';
import { registerDealTools } from './tools/deals.js';

const server = new McpServer({
  name: 'sp-global-data',
  version: '1.0.0',
});

// Company tools (4)
registerCompanyTools(server);

// Financial tools (3)
registerFinancialTools(server);

// Research tools (5)
registerResearchTools(server);

// Deal tools (2)
registerDealTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
