#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerFundTools } from './tools/fund.js';
import { registerEtfTools } from './tools/etf.js';
import { registerResearchTools } from './tools/research.js';
import { registerPortfolioTools } from './tools/portfolio.js';

const server = new McpServer({
  name: 'morningstar-data',
  version: '1.0.0',
});

// Fund tools (5)
registerFundTools(server);

// ETF tools (1)
registerEtfTools(server);

// Research tools (5)
registerResearchTools(server);

// Portfolio tools (3)
registerPortfolioTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
