import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

const CountryDateSchema = z.object({
  country: z.string().min(1).describe('Country code (ISO2 or ISO3, e.g. US, GBR, CN)'),
  date: z.string().optional().describe('Date range (e.g. 2010:2023)'),
});

const CountryOnlySchema = z.object({
  country: z.string().min(1).describe('Country code (ISO2 or ISO3, e.g. US, GBR, CN)'),
});

interface WbRecord {
  indicator?: { id?: string; value?: string };
  country?: { id?: string; value?: string };
  date?: string;
  value?: number | null;
}

function extractRecords(raw: unknown): WbRecord[] {
  if (Array.isArray(raw) && raw.length >= 2 && Array.isArray(raw[1])) {
    return raw[1] as WbRecord[];
  }
  return [];
}

/** Fetch multiple indicators for a country and return grouped by indicator code */
async function fetchIndicators(
  country: string,
  indicators: string[],
  cacheTtl: number,
  date?: string,
): Promise<Record<string, Array<{ year: string; value: number | null }>>> {
  const joined = indicators.join(';');
  const params: Record<string, string | number> = { per_page: 500 };
  if (date) params.date = date;

  const raw = await wbFetch(
    `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(joined)}`,
    params,
    { cacheTtl },
  );
  const records = extractRecords(raw);

  const grouped: Record<string, Array<{ year: string; value: number | null }>> = {};
  for (const code of indicators) grouped[code] = [];

  for (const rec of records) {
    const id = rec.indicator?.id;
    const year = rec.date;
    if (!id || !year || !grouped[id]) continue;
    grouped[id].push({ year, value: rec.value ?? null });
  }

  // Sort each group chronologically
  for (const code of indicators) {
    grouped[code].sort((a, b) => a.year.localeCompare(b.year));
  }

  return grouped;
}

/** Get latest non-null value from a sorted array */
function latestValue(series: Array<{ year: string; value: number | null }>): { year: string; value: number | null } | null {
  for (let i = series.length - 1; i >= 0; i--) {
    if (series[i].value !== null) return series[i];
  }
  return series.length > 0 ? series[series.length - 1] : null;
}

/** Compute simple trend: average annual change over the last N years of data */
function computeTrend(series: Array<{ year: string; value: number | null }>, years: number): number | null {
  const valid = series.filter(s => s.value !== null);
  if (valid.length < 2) return null;
  const recent = valid.slice(-years);
  if (recent.length < 2) return null;
  const first = recent[0].value!;
  const last = recent[recent.length - 1].value!;
  const span = parseInt(recent[recent.length - 1].year) - parseInt(recent[0].year);
  if (span === 0) return null;
  return (last - first) / span;
}

export function registerDevelopmentTools(server: McpServer) {
  // --- Climate ---
  server.tool(
    'wb_climate',
    'Get climate and environment indicators for a country: CO2 emissions per capita, forest area %, renewable energy %, and methane emissions.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'EN.ATM.CO2E.PC',   // CO2 per capita (metric tons)
        'AG.LND.FRST.ZS',   // Forest area (% of land area)
        'EG.FEC.RNEW.ZS',   // Renewable energy (% of total final energy)
        'EN.ATM.METH.KT.CE', // Methane emissions (kt of CO2 equivalent)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        co2_per_capita: data['EN.ATM.CO2E.PC'],
        forest_area_pct: data['AG.LND.FRST.ZS'],
        renewable_energy_pct: data['EG.FEC.RNEW.ZS'],
        methane_kt: data['EN.ATM.METH.KT.CE'],
      });
    },
  );

  server.tool(
    'wb_climate_vulnerability',
    'Composite climate vulnerability view for a country. Computes trends over last 10 years for CO2 trajectory, deforestation rate, renewable transition, and agricultural land change.',
    CountryOnlySchema.shape,
    async (params) => {
      const { country } = CountryOnlySchema.parse(params);
      const indicators = [
        'EN.ATM.CO2E.PC',   // CO2 per capita
        'AG.LND.FRST.ZS',   // Forest area %
        'EG.FEC.RNEW.ZS',   // Renewable energy %
        'AG.LND.AGRI.ZS',   // Agricultural land %
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG);

      const co2Series = data['EN.ATM.CO2E.PC'];
      const forestSeries = data['AG.LND.FRST.ZS'];
      const renewableSeries = data['EG.FEC.RNEW.ZS'];
      const agriSeries = data['AG.LND.AGRI.ZS'];

      return wrapResponse({
        country,
        co2_trajectory: {
          latest: latestValue(co2Series),
          annual_change: computeTrend(co2Series, 10),
        },
        deforestation: {
          latest: latestValue(forestSeries),
          annual_change: computeTrend(forestSeries, 10),
        },
        renewable_transition: {
          latest: latestValue(renewableSeries),
          annual_change: computeTrend(renewableSeries, 10),
        },
        agricultural_land: {
          latest: latestValue(agriSeries),
          annual_change: computeTrend(agriSeries, 10),
        },
      });
    },
  );

  // --- Poverty ---
  server.tool(
    'wb_poverty',
    'Get poverty indicators for a country: poverty headcount ($2.15/day), Gini index, and income share of the bottom 20%.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'SI.POV.DDAY',      // Poverty headcount at $2.15/day (2017 PPP) %
        'SI.POV.GINI',      // Gini index
        'SI.DST.FRST.20',   // Income share held by lowest 20%
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        poverty_headcount_pct: data['SI.POV.DDAY'],
        gini_index: data['SI.POV.GINI'],
        income_share_bottom_20: data['SI.DST.FRST.20'],
      });
    },
  );

  // --- Inequality ---
  server.tool(
    'wb_inequality',
    'Get extended inequality indicators for a country: Gini index, income distribution by quintile, and poverty gap.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'SI.POV.GINI',      // Gini index
        'SI.DST.FRST.20',   // Income share lowest 20%
        'SI.DST.02ND.20',   // Income share second 20%
        'SI.DST.03RD.20',   // Income share third 20%
        'SI.DST.04TH.20',   // Income share fourth 20%
        'SI.DST.05TH.20',   // Income share highest 20%
        'SI.POV.GAPS',      // Poverty gap at $2.15/day (%)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        gini_index: data['SI.POV.GINI'],
        income_quintiles: {
          lowest_20: data['SI.DST.FRST.20'],
          second_20: data['SI.DST.02ND.20'],
          third_20: data['SI.DST.03RD.20'],
          fourth_20: data['SI.DST.04TH.20'],
          highest_20: data['SI.DST.05TH.20'],
        },
        poverty_gap: data['SI.POV.GAPS'],
      });
    },
  );

  // --- Health ---
  server.tool(
    'wb_health',
    'Get health indicators for a country: life expectancy at birth, under-5 mortality rate, and health expenditure as % of GDP.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'SP.DYN.LE00.IN',   // Life expectancy at birth (years)
        'SH.DYN.MORT',      // Under-5 mortality rate (per 1,000 live births)
        'SH.XPD.CHEX.GD.ZS', // Health expenditure (% of GDP)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        life_expectancy: data['SP.DYN.LE00.IN'],
        under5_mortality: data['SH.DYN.MORT'],
        health_expenditure_gdp_pct: data['SH.XPD.CHEX.GD.ZS'],
      });
    },
  );

  // --- Education ---
  server.tool(
    'wb_education',
    'Get education indicators for a country: primary school enrollment rate, adult literacy rate, and R&D expenditure as % of GDP.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'SE.PRM.ENRR',      // Primary enrollment (% gross)
        'SE.ADT.LITR.ZS',   // Adult literacy rate (% ages 15+)
        'GB.XPD.RSDV.GD.ZS', // R&D expenditure (% of GDP)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        primary_enrollment_pct: data['SE.PRM.ENRR'],
        adult_literacy_pct: data['SE.ADT.LITR.ZS'],
        rd_expenditure_gdp_pct: data['GB.XPD.RSDV.GD.ZS'],
      });
    },
  );

  // --- Infrastructure ---
  server.tool(
    'wb_infrastructure',
    'Get infrastructure indicators for a country: electricity access %, internet users %, and mobile subscriptions per 100 people.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'EG.ELC.ACCS.ZS',   // Access to electricity (% of population)
        'IT.NET.USER.ZS',   // Internet users (% of population)
        'IT.CEL.SETS.P2',   // Mobile cellular subscriptions (per 100 people)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        electricity_access_pct: data['EG.ELC.ACCS.ZS'],
        internet_users_pct: data['IT.NET.USER.ZS'],
        mobile_subscriptions_per100: data['IT.CEL.SETS.P2'],
      });
    },
  );

  // --- Logistics ---
  server.tool(
    'wb_logistics',
    'Get Logistics Performance Index (LPI) for a country: overall score, customs, infrastructure, logistics competence, tracking, and timeliness.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'LP.LPI.OVRL.XQ',   // Overall LPI score
        'LP.LPI.CUST.XQ',   // Customs
        'LP.LPI.INFR.XQ',   // Infrastructure
        'LP.LPI.LOGS.XQ',   // Logistics competence
        'LP.LPI.TRAK.XQ',   // Tracking and tracing
        'LP.LPI.TIME.XQ',   // Timeliness
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        overall_lpi: data['LP.LPI.OVRL.XQ'],
        customs: data['LP.LPI.CUST.XQ'],
        infrastructure: data['LP.LPI.INFR.XQ'],
        logistics_competence: data['LP.LPI.LOGS.XQ'],
        tracking: data['LP.LPI.TRAK.XQ'],
        timeliness: data['LP.LPI.TIME.XQ'],
      });
    },
  );

  // --- Trade ---
  server.tool(
    'wb_trade',
    'Get trade indicators for a country: merchandise trade % of GDP, trade openness, FDI net inflows % of GDP, and current account balance % of GDP.',
    CountryDateSchema.shape,
    async (params) => {
      const { country, date } = CountryDateSchema.parse(params);
      const indicators = [
        'TG.VAL.TOTL.GD.ZS',  // Merchandise trade (% of GDP)
        'NE.TRD.GNFS.ZS',     // Trade openness (% of GDP)
        'BX.KLT.DINV.WD.GD.ZS', // FDI net inflows (% of GDP)
        'BN.CAB.XOKA.GD.ZS',  // Current account balance (% of GDP)
      ];
      const data = await fetchIndicators(country, indicators, CacheTTL.LONG, date);
      return wrapResponse({
        country,
        merchandise_trade_gdp_pct: data['TG.VAL.TOTL.GD.ZS'],
        trade_openness_gdp_pct: data['NE.TRD.GNFS.ZS'],
        fdi_net_inflows_gdp_pct: data['BX.KLT.DINV.WD.GD.ZS'],
        current_account_gdp_pct: data['BN.CAB.XOKA.GD.ZS'],
      });
    },
  );
}
