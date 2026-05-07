import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { chronographFetch, CacheTTL } from '../client.js';
import {
  ChronographListPortfoliosSchema,
  ChronographGetCompanySchema,
  ChronographCapitalAccountSchema,
  ChronographKpiPullSchema,
} from '../schemas/portfolios.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

const CRED_REQUIRED_BASE = {
  credentials_required: true,
  vendor: 'chronograph',
  env_var: 'CHRONOGRAPH_API_KEY',
  message: 'Set CHRONOGRAPH_API_KEY in environment to enable Chronograph tools. Sign up at https://chronograph.pe or contact sales.',
} as const;

export function registerChronographTools(server: McpServer): void {
  server.tool(
    'chronograph_list_portfolios',
    'List PE / VC portfolios under management. Returns fund-level metadata: fund name, vintage, AUM, portfolio company count. Requires CHRONOGRAPH_API_KEY (paid vendor).',
    ChronographListPortfoliosSchema.shape,
    async (params) => {
      const validated = ChronographListPortfoliosSchema.parse(params);
      if (!process.env.CHRONOGRAPH_API_KEY) {
        return wrapResponse({ ...CRED_REQUIRED_BASE, tool: 'chronograph_list_portfolios', requested_params: validated });
      }
      const data = await chronographFetch('/portfolios', validated, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'chronograph_get_company',
    'Fetch portco metadata and latest operational metrics by company id. Returns company profile, ownership details, and current KPI snapshot. Requires CHRONOGRAPH_API_KEY (paid vendor).',
    ChronographGetCompanySchema.shape,
    async (params) => {
      const validated = ChronographGetCompanySchema.parse(params);
      if (!process.env.CHRONOGRAPH_API_KEY) {
        return wrapResponse({ ...CRED_REQUIRED_BASE, tool: 'chronograph_get_company', requested_params: validated });
      }
      const { company_id, ...queryParams } = validated;
      const data = await chronographFetch(`/companies/${company_id}`, queryParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'chronograph_capital_account',
    'LP capital account statement for a fund: commitments, contributions, distributions, and NAV. Scope to a specific LP or retrieve fund-wide aggregate. Requires CHRONOGRAPH_API_KEY (paid vendor).',
    ChronographCapitalAccountSchema.shape,
    async (params) => {
      const validated = ChronographCapitalAccountSchema.parse(params);
      if (!process.env.CHRONOGRAPH_API_KEY) {
        return wrapResponse({ ...CRED_REQUIRED_BASE, tool: 'chronograph_capital_account', requested_params: validated });
      }
      const { fund_id, ...queryParams } = validated;
      const data = await chronographFetch(`/funds/${fund_id}/capital-account`, queryParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'chronograph_kpi_pull',
    'Pull KPI time series for a portfolio company over a period range. Supports any Chronograph metric code (e.g. arr, cac, gross_margin, nrr, headcount). Requires CHRONOGRAPH_API_KEY (paid vendor).',
    ChronographKpiPullSchema.shape,
    async (params) => {
      const validated = ChronographKpiPullSchema.parse(params);
      if (!process.env.CHRONOGRAPH_API_KEY) {
        return wrapResponse({ ...CRED_REQUIRED_BASE, tool: 'chronograph_kpi_pull', requested_params: validated });
      }
      const { company_id, metric_codes, ...queryParams } = validated;
      const data = await chronographFetch(
        `/companies/${company_id}/kpis`,
        { ...queryParams, metric_codes: metric_codes.join(',') },
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );
}
