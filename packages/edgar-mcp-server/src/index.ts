#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerCompanyFactsTools } from './tools/company-facts.js';
import { registerFilingTools } from './tools/filings.js';
import { registerSearchTools } from './tools/search.js';
import { registerIdentifierTools } from './tools/identifiers.js';

const server = new McpServer({
  name: 'edgar-sec-data',
  version: '1.0.0',
});

// XBRL / Company facts (5 tools)
registerCompanyFactsTools(server);

// Filings and submissions (6 tools)
registerFilingTools(server);

// Full-text search (3 tools)
registerSearchTools(server);

// Identifier resolution (6 tools)
registerIdentifierTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
