#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerFundamentalsTools } from './tools/fundamentals.js';
import { registerPricingTools } from './tools/pricing.js';
import { registerOwnershipTools } from './tools/ownership.js';
import { registerAnalyticsTools } from './tools/analytics.js';
import { registerResearchTools } from './tools/research.js';
import { registerBatchTools } from './tools/batch.js';

const server = new McpServer({
  name: 'factset-data',
  version: '1.0.0',
});

// Fundamentals tools (3)
registerFundamentalsTools(server);

// Pricing tools (2)
registerPricingTools(server);

// Ownership tools (2)
registerOwnershipTools(server);

// Analytics tools (3)
registerAnalyticsTools(server);

// Research tools (5)
registerResearchTools(server);

// Batch tools (1)
registerBatchTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
