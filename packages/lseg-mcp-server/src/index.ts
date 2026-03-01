#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerPricingTools } from './tools/pricing.js';
import { registerResearchTools } from './tools/research.js';
import { registerFixedIncomeTools } from './tools/fixed-income.js';
import { registerReferenceTools } from './tools/reference.js';

const server = new McpServer({
  name: 'lseg-data',
  version: '1.0.0',
});

// Pricing tools (4)
registerPricingTools(server);

// Research tools (6)
registerResearchTools(server);

// Fixed income tools (2)
registerFixedIncomeTools(server);

// Reference tools (3)
registerReferenceTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
