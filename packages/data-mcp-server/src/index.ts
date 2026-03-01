#!/usr/bin/env node
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

// FRED tools (18)
import { registerSeriesTools } from './fred/tools/series.js';
import { registerReleaseTools } from './fred/tools/releases.js';
import { registerCategoryTools } from './fred/tools/categories.js';
import { registerTagTools } from './fred/tools/tags.js';
import { registerYieldCurveTools } from './fred/tools/yield-curve.js';

// EDGAR tools (15)
import { registerCompanyFactsTools } from './edgar/tools/company-facts.js';
import { registerFilingTools } from './edgar/tools/filings.js';
import { registerSearchTools as registerEdgarSearchTools } from './edgar/tools/search.js';
import { registerIdentifierTools } from './edgar/tools/identifiers.js';

// FIGI tools (7)
import { registerMappingTools } from './figi/tools/mapping.js';
import { registerSearchTools as registerFigiSearchTools } from './figi/tools/search.js';

// Yahoo Finance tools (17)
import { registerQuoteTools } from './yf/tools/quotes.js';
import { registerInfoTools } from './yf/tools/info.js';
import { registerFinancialTools } from './yf/tools/financials.js';
import { registerOptionsTools } from './yf/tools/options.js';

// World Bank tools (18)
import { registerDataTools } from './wb/tools/data.js';
import { registerIndicatorTools } from './wb/tools/indicators.js';
import { registerCountryTools } from './wb/tools/countries.js';
import { registerSourceTools } from './wb/tools/sources.js';

const server = new McpServer({
  name: 'data-sources',
  version: '1.0.0',
});

// FRED (18 tools)
registerSeriesTools(server);
registerReleaseTools(server);
registerCategoryTools(server);
registerTagTools(server);
registerYieldCurveTools(server);

// EDGAR (15 tools)
registerCompanyFactsTools(server);
registerFilingTools(server);
registerEdgarSearchTools(server);
registerIdentifierTools(server);

// FIGI (7 tools)
registerMappingTools(server);
registerFigiSearchTools(server);

// Yahoo Finance (17 tools)
registerQuoteTools(server);
registerInfoTools(server);
registerFinancialTools(server);
registerOptionsTools(server);

// World Bank (18 tools)
registerDataTools(server);
registerIndicatorTools(server);
registerCountryTools(server);
registerSourceTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
