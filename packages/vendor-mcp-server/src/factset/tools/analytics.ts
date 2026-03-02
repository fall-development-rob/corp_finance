import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetPost, wrapResponse, CacheTTL } from '../client.js';
import {
  PortfolioAnalyticsSchema,
  RiskModelSchema,
  FactorExposureSchema,
} from '../schemas/analytics.js';

export function registerAnalyticsTools(server: McpServer) {
  server.tool(
    'factset_portfolio_analytics',
    'Run portfolio attribution and performance analytics via FactSet. Returns total return, active return, tracking error, information ratio, and Brinson attribution effects against a benchmark.',
    PortfolioAnalyticsSchema.shape,
    async (params) => {
      const { portfolio_id, benchmark_id, start_date, end_date } = PortfolioAnalyticsSchema.parse(params);
      const body: Record<string, unknown> = { portfolio: { id: portfolio_id } };
      if (benchmark_id) body.benchmark = { id: benchmark_id };
      if (start_date) body.startDate = start_date;
      if (end_date) body.endDate = end_date;
      const data = await factsetPost('analytics/engines/pa/v3/calculations', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_risk_model',
    'Get portfolio risk decomposition by factor from FactSet. Returns total risk, systematic risk, idiosyncratic risk, factor contributions, and marginal risk for each holding.',
    RiskModelSchema.shape,
    async (params) => {
      const { portfolio_id, risk_model } = RiskModelSchema.parse(params);
      const body: Record<string, unknown> = { portfolio: { id: portfolio_id } };
      if (risk_model) body.riskModel = risk_model;
      const data = await factsetPost('analytics/engines/axp/v3/calculations', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_factor_exposure',
    'Get factor exposure for securities from FactSet. Returns exposures to style factors (value, momentum, size, quality, volatility) and industry factors. Use for factor-based analysis.',
    FactorExposureSchema.shape,
    async (params) => {
      const { ids, factor_model } = FactorExposureSchema.parse(params);
      const body: Record<string, unknown> = { ids };
      if (factor_model) body.factorModel = factor_model;
      const data = await factsetPost('formula-api/v1/cross-sectional', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
