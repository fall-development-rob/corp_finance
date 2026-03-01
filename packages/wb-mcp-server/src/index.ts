#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerIndicatorTools } from './tools/indicators.js';
import { registerCountryTools } from './tools/countries.js';
import { registerDataTools } from './tools/data.js';
import { registerSourceTools } from './tools/sources.js';

const server = new McpServer({
  name: 'wb-development-data',
  version: '1.0.0',
});

registerIndicatorTools(server);
registerCountryTools(server);
registerDataTools(server);
registerSourceTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
