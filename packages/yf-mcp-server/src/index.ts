#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerQuoteTools } from './tools/quotes.js';
import { registerOptionsTools } from './tools/options.js';
import { registerFinancialTools } from './tools/financials.js';
import { registerInfoTools } from './tools/info.js';

const server = new McpServer({
  name: 'yf-data',
  version: '1.0.0',
});

// Quote & price tools (5)
registerQuoteTools(server);

// Options tools (3)
registerOptionsTools(server);

// Financial statement tools (4)
registerFinancialTools(server);

// Company info & analyst tools (3)
registerInfoTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
