import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { moodysFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  EconomicForecastSchema,
  CountryRiskSchema,
  IndustryOutlookSchema,
} from '../schemas/economics.js';

export function registerEconomicsTools(server: McpServer) {
  // 1. Economic forecast
  server.tool(
    'moodys_economic_forecast',
    'Get macro economic forecasts by country and indicator from Moody\'s Analytics. Returns historical and projected values for GDP, inflation, unemployment, interest rates, and other macro indicators. Use for economic scenario analysis and credit outlook.',
    EconomicForecastSchema.shape,
    async (params) => {
      const parsed = EconomicForecastSchema.parse(params);
      const data = await moodysFetch(
        'economics/v1/forecasts',
        {
          country: parsed.country,
          indicator: parsed.indicator,
          start_date: parsed.start_date,
          end_date: parsed.end_date,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  // 2. Country risk
  server.tool(
    'moodys_country_risk',
    'Get country risk assessment and sovereign analysis from Moody\'s. Returns sovereign rating, economic resilience score, institutional strength, fiscal capacity, and susceptibility to event risk. Use for sovereign credit analysis and cross-border investment decisions.',
    CountryRiskSchema.shape,
    async (params) => {
      const parsed = CountryRiskSchema.parse(params);
      const data = await moodysFetch(
        'economics/v1/country-risk',
        { country: parsed.country },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  // 3. Industry outlook
  server.tool(
    'moodys_industry_outlook',
    'Get industry outlook and credit trends by sector from Moody\'s. Returns sector-level credit quality assessment, rating migration trends, default outlook, and key risk factors. Use for sector allocation and credit portfolio strategy.',
    IndustryOutlookSchema.shape,
    async (params) => {
      const parsed = IndustryOutlookSchema.parse(params);
      const data = await moodysFetch(
        'economics/v1/industry-outlook',
        {
          industry: parsed.industry,
          region: parsed.region,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );
}
