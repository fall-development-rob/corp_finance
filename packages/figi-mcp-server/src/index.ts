#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerMappingTools } from './tools/mapping.js';
import { registerSearchTools } from './tools/search.js';

const server = new McpServer({
  name: 'figi-identifier-mapping',
  version: '1.0.0',
});

registerMappingTools(server);
registerSearchTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
