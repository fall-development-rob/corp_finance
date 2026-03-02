import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { wtoFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const TariffSchema = z.object({
  reporter_country: z.string().describe('Reporter country ISO code or name (e.g., "US", "840", "United States")'),
  product_code: z.string().optional().describe('HS product code at 2, 4, or 6 digit level (e.g., "01", "0101", "010121")'),
  year: z.number().int().min(1990).max(2030).optional().describe('Year to query (e.g., 2023)'),
});

const BarrierSchema = z.object({
  country: z.string().optional().describe('Notifying member ISO code (e.g., "US", "EU")'),
  keyword: z.string().optional().describe('Keyword to search in notification descriptions'),
});

const TradeStatsSchema = z.object({
  reporter: z.string().describe('Reporter country ISO code (e.g., "US", "840")'),
  partner: z.string().describe('Partner country ISO code (e.g., "CN", "156")'),
  year: z.number().int().min(1990).max(2030).optional().describe('Year to query'),
});

// ---------- Response types ----------

interface WtoDataRecord {
  [key: string]: unknown;
}

// ---------- Registration ----------

export function registerWtoTools(server: McpServer) {
  // --- wto_tariffs ---
  server.tool(
    'wto_tariffs',
    'Query WTO tariff rates by country and product. Returns MFN applied rates and bound rates. Use HS codes to filter by product category.',
    TariffSchema.shape,
    async (params) => {
      const { reporter_country, product_code, year } = TariffSchema.parse(params);

      // WTO Timeseries API: indicator HS_M_0010 = MFN Simple Average
      const queryParams: Record<string, string | number> = {
        i: 'HS_M_0010', // MFN applied tariff indicator
        r: reporter_country,
        fmt: 'json',
        mode: 'full',
        lang: 1, // English
        max: 100,
      };

      if (product_code) queryParams.ps = product_code;
      if (year) queryParams.tp = year;

      const raw = await wtoFetch<WtoDataRecord[]>('data', queryParams, {
        cacheTtl: CacheTTL.MEDIUM,
      });

      const records = (Array.isArray(raw) ? raw : []).map(d => ({
        reporter: d['ReportingEconomyCode'] ?? d['ReportingEconomy'] ?? reporter_country,
        product_code: d['ProductOrSectorCode'] ?? d['ProductOrSector'] ?? product_code ?? null,
        product_description: d['ProductOrSector'] ?? d['ProductOrSectorDescription'] ?? null,
        mfn_applied_rate: d['Value'] ?? null,
        bound_rate: d['BoundRate'] ?? null,
        year: d['Year'] ?? d['TimePeriod'] ?? year ?? null,
      }));

      return wrapResponse({
        source: 'WTO',
        reporter: reporter_country,
        product_code: product_code ?? 'all',
        count: records.length,
        data: records,
      });
    },
  );

  // --- wto_barriers ---
  server.tool(
    'wto_barriers',
    'Query WTO SPS/TBT trade barrier notifications. Filter by notifying country or keyword. Returns notification details including products covered.',
    BarrierSchema.shape,
    async (params) => {
      const { country, keyword } = BarrierSchema.parse(params);

      // SPS notifications indicator
      const spsParams: Record<string, string | number> = {
        i: 'SPS_NTF_0010', // SPS notifications
        fmt: 'json',
        mode: 'full',
        lang: 1,
        max: 50,
      };

      if (country) spsParams.r = country;

      // TBT notifications indicator
      const tbtParams: Record<string, string | number> = {
        i: 'TBT_NTF_0010', // TBT notifications
        fmt: 'json',
        mode: 'full',
        lang: 1,
        max: 50,
      };

      if (country) tbtParams.r = country;

      // Fetch SPS and TBT in parallel
      const [spsRaw, tbtRaw] = await Promise.all([
        wtoFetch<WtoDataRecord[]>('data', spsParams, { cacheTtl: CacheTTL.MEDIUM }).catch(() => [] as WtoDataRecord[]),
        wtoFetch<WtoDataRecord[]>('data', tbtParams, { cacheTtl: CacheTTL.MEDIUM }).catch(() => [] as WtoDataRecord[]),
      ]);

      const mapNotification = (d: WtoDataRecord, type: 'SPS' | 'TBT') => ({
        notification_id: d['IndicatorCode'] ?? d['NotificationId'] ?? null,
        type,
        notifying_member: d['ReportingEconomy'] ?? d['ReportingEconomyCode'] ?? country ?? null,
        date: d['Year'] ?? d['TimePeriod'] ?? null,
        products_covered: d['ProductOrSector'] ?? d['ProductOrSectorDescription'] ?? null,
        description: d['IndicatorDescription'] ?? d['Indicator'] ?? null,
        value: d['Value'] ?? null,
      });

      const spsRecords = (Array.isArray(spsRaw) ? spsRaw : []).map(d => mapNotification(d, 'SPS'));
      const tbtRecords = (Array.isArray(tbtRaw) ? tbtRaw : []).map(d => mapNotification(d, 'TBT'));

      let allRecords = [...spsRecords, ...tbtRecords];

      // Client-side keyword filter if provided
      if (keyword) {
        const lower = keyword.toLowerCase();
        allRecords = allRecords.filter(r =>
          (r.description && String(r.description).toLowerCase().includes(lower)) ||
          (r.products_covered && String(r.products_covered).toLowerCase().includes(lower)) ||
          (r.notifying_member && String(r.notifying_member).toLowerCase().includes(lower))
        );
      }

      return wrapResponse({
        source: 'WTO',
        country: country ?? 'all',
        keyword: keyword ?? null,
        count: allRecords.length,
        data: allRecords,
      });
    },
  );

  // --- wto_trade_stats ---
  server.tool(
    'wto_trade_stats',
    'Query WTO bilateral trade flows between two countries. Returns merchandise exports, imports, and trade balance values.',
    TradeStatsSchema.shape,
    async (params) => {
      const { reporter, partner, year } = TradeStatsSchema.parse(params);

      // Merchandise exports indicator
      const exportParams: Record<string, string | number> = {
        i: 'HS_M_0020', // Merchandise trade values
        r: reporter,
        p: partner,
        fmt: 'json',
        mode: 'full',
        lang: 1,
        max: 50,
      };

      if (year) exportParams.tp = year;

      const raw = await wtoFetch<WtoDataRecord[]>('data', exportParams, {
        cacheTtl: CacheTTL.LONG,
      });

      const rows = Array.isArray(raw) ? raw : [];

      // Separate export and import flows
      const exports = rows.filter(d =>
        String(d['IndicatorCode'] ?? '').includes('X') ||
        String(d['Indicator'] ?? '').toLowerCase().includes('export')
      );
      const imports = rows.filter(d =>
        String(d['IndicatorCode'] ?? '').includes('M') ||
        String(d['Indicator'] ?? '').toLowerCase().includes('import')
      );

      // Build year-keyed records combining exports and imports
      const yearMap = new Map<string, {
        reporter: string;
        partner: string;
        exports_value: unknown;
        imports_value: unknown;
        trade_balance: number | null;
        product_groups: unknown;
      }>();

      for (const d of rows) {
        const yr = String(d['Year'] ?? d['TimePeriod'] ?? 'unknown');
        if (!yearMap.has(yr)) {
          yearMap.set(yr, {
            reporter: String(d['ReportingEconomy'] ?? reporter),
            partner: String(d['PartnerEconomy'] ?? partner),
            exports_value: null,
            imports_value: null,
            trade_balance: null,
            product_groups: d['ProductOrSector'] ?? null,
          });
        }
        const entry = yearMap.get(yr)!;
        const val = Number(d['Value']);
        const isExport = String(d['IndicatorCode'] ?? '').includes('X') ||
          String(d['Indicator'] ?? '').toLowerCase().includes('export');

        if (isExport) {
          entry.exports_value = isNaN(val) ? d['Value'] : val;
        } else {
          entry.imports_value = isNaN(val) ? d['Value'] : val;
        }

        if (typeof entry.exports_value === 'number' && typeof entry.imports_value === 'number') {
          entry.trade_balance = entry.exports_value - entry.imports_value;
        }
      }

      // If no clear export/import distinction, return raw rows
      const records = yearMap.size > 0
        ? Array.from(yearMap.entries()).map(([yr, v]) => ({ year: yr, ...v }))
        : rows.map(d => ({
            reporter: d['ReportingEconomy'] ?? reporter,
            partner: d['PartnerEconomy'] ?? partner,
            exports_value: d['Value'] ?? null,
            imports_value: null,
            trade_balance: null,
            product_groups: d['ProductOrSector'] ?? null,
            year: d['Year'] ?? d['TimePeriod'] ?? null,
          }));

      return wrapResponse({
        source: 'WTO',
        reporter,
        partner,
        count: records.length,
        data: records,
      });
    },
  );
}
