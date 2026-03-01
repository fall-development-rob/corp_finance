#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerCompanyTools } from './tools/company.js';
import { registerDealTools } from './tools/deals.js';
import { registerInvestorTools } from './tools/investors.js';
import { registerMarketTools } from './tools/market.js';

const server = new McpServer({
  name: 'pitchbook-data',
  version: '1.0.0',
});

// Company tools (2)
registerCompanyTools(server);

// Deal tools (3)
registerDealTools(server);

// Investor tools (4)
registerInvestorTools(server);

// Market tools (5)
registerMarketTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
