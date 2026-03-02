import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { usaSpendingFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const ContractsSchema = z.object({
  keyword: z.string().optional().describe('Search keyword for contract descriptions'),
  agency: z.string().optional().describe('Awarding agency name (e.g., "Department of Defense")'),
  amount_min: z.number().optional().describe('Minimum award amount in USD'),
  amount_max: z.number().optional().describe('Maximum award amount in USD'),
  date_range: z.string().optional().describe('Date range as "YYYY-MM-DD,YYYY-MM-DD" — defaults to last 90 days'),
  limit: z.number().int().min(1).max(100).default(25).describe('Number of records to return'),
});

const AgenciesSchema = z.object({
  fiscal_year: z.number().int().min(2000).max(2030).optional()
    .describe('Fiscal year to query (e.g., 2025). Defaults to current fiscal year.'),
});

// ---------- Helpers ----------

function defaultDateRange(): { start: string; end: string } {
  const end = new Date();
  const start = new Date();
  start.setDate(start.getDate() - 90);
  return {
    start: start.toISOString().slice(0, 10),
    end: end.toISOString().slice(0, 10),
  };
}

function currentFiscalYear(): number {
  const now = new Date();
  // US fiscal year starts October 1
  return now.getMonth() >= 9 ? now.getFullYear() + 1 : now.getFullYear();
}

// ---------- Response types ----------

interface SpendingByAwardResponse {
  results: Record<string, unknown>[];
  page_metadata?: {
    total: number;
    page: number;
    hasNext: boolean;
  };
}

interface AgencyListResponse {
  results: Record<string, unknown>[];
}

// ---------- Registration ----------

export function registerUsaSpendingTools(server: McpServer) {
  // --- usaspending_contracts ---
  server.tool(
    'usaspending_contracts',
    'Search US federal contract awards from USASpending.gov. Filter by keyword, agency, amount range, and date range. Returns award details including recipient and obligation amount.',
    ContractsSchema.shape,
    async (params) => {
      const { keyword, agency, amount_min, amount_max, date_range, limit } = ContractsSchema.parse(params);

      // Parse date range
      let startDate: string;
      let endDate: string;
      if (date_range && date_range.includes(',')) {
        [startDate, endDate] = date_range.split(',').map(s => s.trim());
      } else {
        const defaults = defaultDateRange();
        startDate = defaults.start;
        endDate = defaults.end;
      }

      // Build the POST body for spending_by_award
      const filters: Record<string, unknown> = {
        award_type_codes: ['A', 'B', 'C', 'D'], // Contracts only
        time_period: [{ start_date: startDate, end_date: endDate }],
      };

      if (keyword) {
        filters.keywords = [keyword];
      }

      if (agency) {
        filters.agencies = [{
          type: 'awarding',
          tier: 'toptier',
          name: agency,
        }];
      }

      if (amount_min !== undefined || amount_max !== undefined) {
        const range: Record<string, number> = {};
        if (amount_min !== undefined) range.lower_bound = amount_min;
        if (amount_max !== undefined) range.upper_bound = amount_max;
        filters.award_amounts = [range];
      }

      const body = {
        filters,
        fields: [
          'Award ID',
          'Recipient Name',
          'Total Obligation',
          'Awarding Agency',
          'Award Type',
          'Start Date',
          'Description',
        ],
        page: 1,
        limit,
        sort: 'Total Obligation',
        order: 'desc',
      };

      const raw = await usaSpendingFetch<SpendingByAwardResponse>(
        'search/spending_by_award/',
        body,
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const records = (raw.results ?? []).map(d => ({
        award_id: d['Award ID'] ?? d['internal_id'] ?? null,
        recipient_name: d['Recipient Name'] ?? d['recipient_name'] ?? null,
        total_obligation: d['Total Obligation'] ?? d['total_obligation'] ?? null,
        awarding_agency: d['Awarding Agency'] ?? d['awarding_agency'] ?? null,
        award_type: d['Award Type'] ?? d['award_type'] ?? null,
        period_of_performance_start: d['Start Date'] ?? d['start_date'] ?? null,
        description: d['Description'] ?? d['description'] ?? null,
      }));

      return wrapResponse({
        source: 'USASpending',
        filters: {
          keyword: keyword ?? null,
          agency: agency ?? null,
          amount_min: amount_min ?? null,
          amount_max: amount_max ?? null,
          date_range: `${startDate} to ${endDate}`,
        },
        count: records.length,
        total: raw.page_metadata?.total ?? records.length,
        data: records,
      });
    },
  );

  // --- usaspending_agencies ---
  server.tool(
    'usaspending_agencies',
    'Get US federal agency spending summaries from USASpending.gov. Returns total budgetary resources and obligations by top-tier agency for a fiscal year.',
    AgenciesSchema.shape,
    async (params) => {
      const { fiscal_year } = AgenciesSchema.parse(params);
      const fy = fiscal_year ?? currentFiscalYear();

      // GET endpoint for top-tier agency list
      const raw = await usaSpendingFetch<AgencyListResponse>(
        `agency/toptier/?sort=total_budgetary_resources&order=desc&fiscal_year=${fy}`,
        undefined,
        { cacheTtl: CacheTTL.LONG },
      );

      const records = (raw.results ?? []).map(d => ({
        agency_name: d['agency_name'] ?? d['name'] ?? null,
        total_budgetary_resources: d['total_budgetary_resources'] ?? d['budget_authority_amount'] ?? null,
        total_obligations: d['total_obligations'] ?? d['obligated_amount'] ?? null,
        agency_code: d['toptier_code'] ?? d['agency_code'] ?? null,
      }));

      return wrapResponse({
        source: 'USASpending',
        fiscal_year: fy,
        count: records.length,
        data: records,
      });
    },
  );
}
