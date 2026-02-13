#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { registerQuoteTools } from './tools/quotes.js';
import { registerProfileTools } from './tools/profiles.js';
import { registerFinancialTools } from './tools/financials.js';
import { registerEarningsTools } from './tools/earnings.js';
import { registerMarketTools } from './tools/market.js';
import { registerEtfTools } from './tools/etf.js';
import { registerNewsTools } from './tools/news.js';
import { registerTechnicalTools } from './tools/technicals.js';
import { registerSecTools } from './tools/sec.js';
import { registerInsiderTools } from './tools/insider.js';
import { registerInstitutionalTools } from './tools/institutional.js';
import { registerDividendTools } from './tools/dividends.js';
import { registerFinancialExtendedTools } from './tools/financials-extended.js';
import { registerMarketExtendedTools } from './tools/market-extended.js';
import { registerCompanyExtendedTools } from './tools/company-extended.js';

const server = new McpServer({
  name: 'fmp-market-data',
  version: '2.0.0',
});

// Core tools
registerQuoteTools(server);
registerProfileTools(server);
registerFinancialTools(server);
registerEarningsTools(server);
registerMarketTools(server);

// Extended tools
registerEtfTools(server);
registerNewsTools(server);
registerTechnicalTools(server);
registerSecTools(server);
registerInsiderTools(server);
registerInstitutionalTools(server);
registerDividendTools(server);
registerFinancialExtendedTools(server);
registerMarketExtendedTools(server);
registerCompanyExtendedTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
