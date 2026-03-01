#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerSeriesTools } from './tools/series.js';
import { registerReleaseTools } from './tools/releases.js';
import { registerCategoryTools } from './tools/categories.js';
import { registerTagTools } from './tools/tags.js';
import { registerYieldCurveTools } from './tools/yield-curve.js';

const server = new McpServer({
  name: 'fred-data',
  version: '1.0.0',
});

// Series tools (6)
registerSeriesTools(server);

// Release tools (4)
registerReleaseTools(server);

// Category tools (3)
registerCategoryTools(server);

// Tag tools (3)
registerTagTools(server);

// Yield curve tools (2)
registerYieldCurveTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
