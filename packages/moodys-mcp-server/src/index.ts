#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerRatingsTools } from './tools/ratings.js';
import { registerDefaultsTools } from './tools/defaults.js';
import { registerEconomicsTools } from './tools/economics.js';
import { registerEsgTools } from './tools/esg.js';
import { registerStructuredTools } from './tools/structured.js';

const server = new McpServer({
  name: 'moodys-data',
  version: '1.0.0',
});

// Ratings tools (3)
registerRatingsTools(server);

// Defaults tools (3)
registerDefaultsTools(server);

// Economics tools (3)
registerEconomicsTools(server);

// ESG tools (2)
registerEsgTools(server);

// Structured tools (3)
registerStructuredTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
