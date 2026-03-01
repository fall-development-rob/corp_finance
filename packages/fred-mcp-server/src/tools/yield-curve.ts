import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fredFetch, CacheTTL } from '../client.js';
import { YieldCurveSchema, SpreadSchema } from '../schemas/yield-curve.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

interface FredObservation {
  date: string;
  value: string;
}

interface FredObservationsResponse {
  observations: FredObservation[];
}

const YIELD_CURVE_TENORS = [
  { series_id: 'DGS1MO', label: '1 Month', months: 1 },
  { series_id: 'DGS3MO', label: '3 Month', months: 3 },
  { series_id: 'DGS6MO', label: '6 Month', months: 6 },
  { series_id: 'DGS1',   label: '1 Year',  months: 12 },
  { series_id: 'DGS2',   label: '2 Year',  months: 24 },
  { series_id: 'DGS3',   label: '3 Year',  months: 36 },
  { series_id: 'DGS5',   label: '5 Year',  months: 60 },
  { series_id: 'DGS7',   label: '7 Year',  months: 84 },
  { series_id: 'DGS10',  label: '10 Year', months: 120 },
  { series_id: 'DGS20',  label: '20 Year', months: 240 },
  { series_id: 'DGS30',  label: '30 Year', months: 360 },
];

export function registerYieldCurveTools(server: McpServer) {
  server.tool(
    'fred_yield_curve',
    'Get the US Treasury yield curve by fetching rates for all standard maturities (1M to 30Y) in parallel. Returns structured curve with date, tenors, labels, and rates. Essential for fixed income analysis, term structure modelling, and WACC risk-free rate selection.',
    YieldCurveSchema.shape,
    async (params) => {
      const { observation_start, observation_end, limit } = YieldCurveSchema.parse(params);

      // Fetch all tenors in parallel
      const results = await Promise.all(
        YIELD_CURVE_TENORS.map(async (tenor) => {
          const resp = await fredFetch<FredObservationsResponse>('series/observations', {
            series_id: tenor.series_id,
            observation_start,
            observation_end,
            limit,
            sort_order: 'desc',
          }, { cacheTtl: CacheTTL.SHORT });
          return { tenor, observations: resp.observations ?? [] };
        }),
      );

      // Group by date — take the most recent date that has data across tenors
      const dateMap = new Map<string, Array<{ label: string; months: number; rate: number | null; series_id: string }>>();

      for (const { tenor, observations } of results) {
        for (const obs of observations) {
          if (!dateMap.has(obs.date)) dateMap.set(obs.date, []);
          const rate = obs.value === '.' ? null : parseFloat(obs.value);
          dateMap.get(obs.date)!.push({
            label: tenor.label,
            months: tenor.months,
            rate,
            series_id: tenor.series_id,
          });
        }
      }

      // Sort dates descending and take requested number
      const sortedDates = Array.from(dateMap.keys()).sort().reverse().slice(0, limit);

      const curves = sortedDates.map(date => {
        const tenors = dateMap.get(date)!
          .sort((a, b) => a.months - b.months);
        return { date, tenors };
      });

      return wrapResponse({ count: curves.length, curves });
    },
  );

  server.tool(
    'fred_spread',
    'Compute the spread between two FRED series (long minus short). Use for yield curve slope (e.g., DGS10 - DGS2), credit spreads, or any rate differential analysis. Returns date-aligned spread time series.',
    SpreadSchema.shape,
    async (params) => {
      const { series_id_long, series_id_short, observation_start, observation_end, limit } = SpreadSchema.parse(params);

      // Fetch both series in parallel
      const [longResp, shortResp] = await Promise.all([
        fredFetch<FredObservationsResponse>('series/observations', {
          series_id: series_id_long,
          observation_start,
          observation_end,
          limit,
          sort_order: 'desc',
        }, { cacheTtl: CacheTTL.SHORT }),
        fredFetch<FredObservationsResponse>('series/observations', {
          series_id: series_id_short,
          observation_start,
          observation_end,
          limit,
          sort_order: 'desc',
        }, { cacheTtl: CacheTTL.SHORT }),
      ]);

      // Build lookup for short series by date
      const shortByDate = new Map<string, string>();
      for (const obs of (shortResp.observations ?? [])) {
        shortByDate.set(obs.date, obs.value);
      }

      // Compute spread for matching dates
      const spreads: Array<{ date: string; long_rate: number | null; short_rate: number | null; spread: number | null }> = [];

      for (const obs of (longResp.observations ?? [])) {
        const shortValue = shortByDate.get(obs.date);
        if (shortValue === undefined) continue;

        const longRate = obs.value === '.' ? null : parseFloat(obs.value);
        const shortRate = shortValue === '.' ? null : parseFloat(shortValue);
        const spread = (longRate !== null && shortRate !== null) ? Math.round((longRate - shortRate) * 10000) / 10000 : null;

        spreads.push({
          date: obs.date,
          long_rate: longRate,
          short_rate: shortRate,
          spread,
        });
      }

      return wrapResponse({
        series_long: series_id_long,
        series_short: series_id_short,
        unit: 'percentage points',
        count: spreads.length,
        observations: spreads,
      });
    },
  );
}
