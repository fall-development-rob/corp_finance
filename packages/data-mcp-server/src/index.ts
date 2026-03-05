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

// World Bank tools (18 + 12 extended)
import { registerDataTools } from './wb/tools/data.js';
import { registerIndicatorTools } from './wb/tools/indicators.js';
import { registerCountryTools } from './wb/tools/countries.js';
import { registerSourceTools } from './wb/tools/sources.js';
import { registerGovernanceTools } from './wb/tools/governance.js';
import { registerDevelopmentTools } from './wb/tools/development.js';

// Conflict tools (9)
import { registerAcledTools } from './conflict/acled/tools.js';
import { registerUcdpTools } from './conflict/ucdp/tools.js';
import { registerGdeltTools } from './conflict/gdelt/tools.js';

// Environment tools (9)
import { registerGdacsTools } from './environment/gdacs/tools.js';
import { registerUsgsTools } from './environment/usgs/tools.js';
import { registerFirmsTools } from './environment/nasa/firms-tools.js';
import { registerEonetTools } from './environment/nasa/eonet-tools.js';

// Trade tools (8)
import { registerEiaTools } from './trade/eia/tools.js';
import { registerWtoTools } from './trade/wto/tools.js';
import { registerUsaSpendingTools } from './trade/usaspending/tools.js';

// Alternative data tools (8)
import { registerPolymarketTools } from './alternative/polymarket/tools.js';
import { registerCoinGeckoTools } from './alternative/coingecko/tools.js';
import { registerUnhcrTools } from './alternative/unhcr/tools.js';
import { registerOpenMeteoTools } from './alternative/openmeteo/tools.js';

// Alpha Vantage tools (36)
import { registerQuoteTools as registerAvQuoteTools } from './alphavantage/tools/quotes.js';
import { registerTimeSeriesTools } from './alphavantage/tools/time-series.js';
import { registerFundamentalTools } from './alphavantage/tools/fundamentals.js';
import { registerForexCryptoTools } from './alphavantage/tools/forex-crypto.js';
import { registerEconomicsTools } from './alphavantage/tools/economics.js';
import { registerTechnicalTools as registerAvTechnicalTools } from './alphavantage/tools/technicals.js';
import { registerIntelligenceTools } from './alphavantage/tools/intelligence.js';

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

// World Bank (18 + 12 extended = 30 tools)
registerDataTools(server);
registerIndicatorTools(server);
registerCountryTools(server);
registerSourceTools(server);
registerGovernanceTools(server);
registerDevelopmentTools(server);

// Conflict (9 tools)
registerAcledTools(server);
registerUcdpTools(server);
registerGdeltTools(server);

// Environment (9 tools)
registerGdacsTools(server);
registerUsgsTools(server);
registerFirmsTools(server);
registerEonetTools(server);

// Trade (8 tools)
registerEiaTools(server);
registerWtoTools(server);
registerUsaSpendingTools(server);

// Alternative data (8 tools)
registerPolymarketTools(server);
registerCoinGeckoTools(server);
registerUnhcrTools(server);
registerOpenMeteoTools(server);

// Alpha Vantage (36 tools)
registerAvQuoteTools(server);
registerTimeSeriesTools(server);
registerFundamentalTools(server);
registerForexCryptoTools(server);
registerEconomicsTools(server);
registerAvTechnicalTools(server);
registerIntelligenceTools(server);

const transport = new StdioServerTransport();
await server.connect(transport);
