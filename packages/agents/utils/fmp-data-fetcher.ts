// FMP Data Fetcher — resolves symbols, fetches financials, maps to ExtractedMetrics
// Bridges live FMP market data into the agent analytical pipeline

import type { ExtractedMetrics } from './financial-parser.js';

type FmpCaller = (toolName: string, params: Record<string, unknown>) => Promise<unknown>;

/**
 * Resolve a company name to a ticker symbol via FMP search.
 * Returns null if no match found.
 */
export async function resolveSymbol(
  companyName: string,
  callFmp: FmpCaller,
): Promise<string | null> {
  try {
    const results = await callFmp('fmp_search_name', { query: companyName, limit: 5 });
    if (!Array.isArray(results) || results.length === 0) return null;

    // Prefer exact or close name match on a major exchange
    const majorExchanges = new Set(['NYSE', 'NASDAQ', 'AMEX', 'LSE', 'TSX']);
    const preferred = results.find(
      (r: any) => majorExchanges.has(r.exchangeShortName) && r.name?.toLowerCase().includes(companyName.toLowerCase().split(' ')[0]),
    );
    return (preferred?.symbol ?? results[0]?.symbol) || null;
  } catch {
    return null;
  }
}

/** Raw FMP data from parallel fetches */
export interface FmpRawData {
  income?: Record<string, unknown>;
  balanceSheet?: Record<string, unknown>;
  cashFlow?: Record<string, unknown>;
  keyMetrics?: Record<string, unknown>;
  profile?: Record<string, unknown>;
  quote?: Record<string, unknown>;
}

/**
 * Fetch core financial data from FMP in parallel (6 calls).
 * Each call is independently error-tolerant.
 */
export async function fetchMarketData(
  symbol: string,
  callFmp: FmpCaller,
): Promise<FmpRawData> {
  const [income, balanceSheet, cashFlow, keyMetrics, profile, quote] = await Promise.allSettled([
    callFmp('fmp_income_statement', { symbol, period: 'annual', limit: 1 }),
    callFmp('fmp_balance_sheet', { symbol, period: 'annual', limit: 1 }),
    callFmp('fmp_cash_flow', { symbol, period: 'annual', limit: 1 }),
    callFmp('fmp_key_metrics', { symbol, period: 'annual', limit: 1 }),
    callFmp('fmp_company_profile', { symbol }),
    callFmp('fmp_quote', { symbol }),
  ]);

  const extract = (result: PromiseSettledResult<unknown>): Record<string, unknown> | undefined => {
    if (result.status !== 'fulfilled') return undefined;
    const val = result.value;
    if (Array.isArray(val) && val.length > 0) return val[0] as Record<string, unknown>;
    if (val && typeof val === 'object' && !Array.isArray(val)) return val as Record<string, unknown>;
    return undefined;
  };

  return {
    income: extract(income),
    balanceSheet: extract(balanceSheet),
    cashFlow: extract(cashFlow),
    keyMetrics: extract(keyMetrics),
    profile: extract(profile),
    quote: extract(quote),
  };
}

/** Safely get a numeric value from an object */
function num(obj: Record<string, unknown> | undefined, key: string): number | undefined {
  if (!obj) return undefined;
  const v = obj[key];
  if (typeof v === 'number' && !isNaN(v)) return v;
  if (typeof v === 'string') {
    const n = parseFloat(v);
    if (!isNaN(n)) return n;
  }
  return undefined;
}

/** Safely get a string value from an object */
function str(obj: Record<string, unknown> | undefined, key: string): string | undefined {
  if (!obj) return undefined;
  const v = obj[key];
  return typeof v === 'string' ? v : undefined;
}

/**
 * Map raw FMP data to ExtractedMetrics fields.
 */
export function mapFmpToMetrics(fmp: FmpRawData, symbol: string): Partial<ExtractedMetrics> {
  const m: Partial<ExtractedMetrics> = {};
  const { income, balanceSheet, cashFlow, keyMetrics, profile, quote } = fmp;

  // Symbol + company info
  m._symbol = symbol;
  m._company = str(profile, 'companyName') ?? str(income, 'symbol');
  m._sector = str(profile, 'sector');
  m._industry = str(profile, 'industry');

  // Income statement
  m.revenue = num(income, 'revenue');
  m.ebitda = num(income, 'ebitda');
  m.ebit = num(income, 'operatingIncome');
  m.net_income = num(income, 'netIncome');
  m.cogs = num(income, 'costOfRevenue');
  m.sga = num(income, 'sellingGeneralAndAdministrativeExpenses');
  m.eps = num(income, 'eps');
  m.interest_expense = num(income, 'interestExpense');
  m.depreciation = num(income, 'depreciationAndAmortization');

  // Balance sheet
  m.total_assets = num(balanceSheet, 'totalAssets');
  m.total_equity = num(balanceSheet, 'totalStockholdersEquity');
  m.total_debt = num(balanceSheet, 'totalDebt');
  m.net_debt = num(balanceSheet, 'netDebt');
  m.cash = num(balanceSheet, 'cashAndCashEquivalents');
  m.current_assets = num(balanceSheet, 'totalCurrentAssets');
  m.current_liabilities = num(balanceSheet, 'totalCurrentLiabilities');
  m.receivables = num(balanceSheet, 'netReceivables');
  m.inventory = num(balanceSheet, 'inventory');
  m.payables = num(balanceSheet, 'accountPayables');
  m.ppe = num(balanceSheet, 'propertyPlantEquipmentNet');

  // Cash flow
  m.operating_cash_flow = num(cashFlow, 'operatingCashFlow');
  const rawCapex = num(cashFlow, 'capitalExpenditure');
  m.capex = rawCapex !== undefined ? Math.abs(rawCapex) : undefined;

  // Key metrics
  m.debt_to_equity = num(keyMetrics, 'debtToEquity');
  m.current_ratio = num(keyMetrics, 'currentRatio');
  m.interest_coverage = num(keyMetrics, 'interestCoverage');
  m.enterprise_value = num(keyMetrics, 'enterpriseValue');

  // Profile
  m.beta = num(profile, 'beta');
  m.market_cap = num(profile, 'mktCap');
  m.dividend_per_share = num(profile, 'lastDiv');
  m.shares_outstanding = num(quote, 'sharesOutstanding') ?? num(income, 'weightedAverageShsOut');

  // Quote
  m.share_price = num(quote, 'price');

  // Derived margins
  if (m.revenue && m.revenue > 0) {
    if (m.ebitda) m.ebitda_margin = m.ebitda / m.revenue;
    if (m.ebit) m.ebit_margin = m.ebit / m.revenue;
    if (m.net_income) m.net_margin = m.net_income / m.revenue;
    if (m.cogs) m.gross_margin = (m.revenue - m.cogs) / m.revenue;
  }

  // Derived ratios (if not from key_metrics)
  if (!m.interest_coverage && m.ebitda && m.interest_expense && m.interest_expense > 0) {
    m.interest_coverage = m.ebitda / m.interest_expense;
  }
  if (!m.debt_to_equity && m.total_debt && m.total_equity && m.total_equity > 0) {
    m.debt_to_equity = m.total_debt / m.total_equity;
  }

  return m;
}

/**
 * Merge text-parsed metrics with FMP-fetched metrics.
 * Text-parsed values take priority (explicit user input overrides market data).
 */
export function mergeMetrics(
  textMetrics: ExtractedMetrics,
  fmpMetrics: Partial<ExtractedMetrics>,
): ExtractedMetrics {
  const merged: ExtractedMetrics = { ...textMetrics, _dataSource: 'fmp-enriched' as const };

  for (const [key, value] of Object.entries(fmpMetrics)) {
    if (value === undefined || value === null) continue;

    const k = key as keyof ExtractedMetrics;
    // Text-parsed value takes priority — user explicitly provided it
    if (k === '_raw') continue; // never overwrite raw text
    if (merged[k] !== undefined && k !== '_symbol' && k !== '_sector' && k !== '_industry' && k !== '_dataSource' && k !== '_company') continue;

    (merged as unknown as Record<string, unknown>)[k] = value;
  }

  return merged;
}

/**
 * Full enrichment pipeline: resolve symbol -> fetch data -> map -> merge.
 * Gracefully falls back to text-only metrics if FMP is unavailable.
 */
export async function enrichMetrics(
  textMetrics: ExtractedMetrics,
  callFmp: FmpCaller,
): Promise<ExtractedMetrics> {
  const companyName = textMetrics._company;
  if (!companyName) return { ...textMetrics, _dataSource: 'text-only' };

  try {
    // Check if user already provided a ticker-like string (all caps, 1-5 chars)
    const tickerMatch = textMetrics._raw.match(/\b([A-Z]{1,5})\b/);
    let symbol: string | null = null;

    // Try direct symbol first if it looks like a ticker
    if (tickerMatch && tickerMatch[1].length <= 5) {
      symbol = tickerMatch[1];
    }

    // Fall back to name search
    if (!symbol) {
      symbol = await resolveSymbol(companyName, callFmp);
    }

    if (!symbol) return { ...textMetrics, _dataSource: 'text-only' };

    const fmpData = await fetchMarketData(symbol, callFmp);
    const fmpMetrics = mapFmpToMetrics(fmpData, symbol);
    return mergeMetrics(textMetrics, fmpMetrics);
  } catch {
    return { ...textMetrics, _dataSource: 'text-only' };
  }
}
