import { describe, it, expect, vi } from 'vitest';
import {
  resolveSymbol,
  fetchMarketData,
  mapFmpToMetrics,
  mergeMetrics,
  enrichMetrics,
  type FmpRawData,
} from '../utils/fmp-data-fetcher.js';
import type { ExtractedMetrics } from '../utils/financial-parser.js';

// ─── resolveSymbol ──────────────────────────────────────────────────────────

describe('resolveSymbol', () => {
  it('returns the best ticker from search results', async () => {
    const callFmp = vi.fn().mockResolvedValue([
      { symbol: 'AAPL', name: 'Apple Inc.', exchangeShortName: 'NASDAQ' },
      { symbol: 'APLE', name: 'Apple Hospitality REIT', exchangeShortName: 'NYSE' },
    ]);
    const result = await resolveSymbol('Apple', callFmp);
    expect(result).toBe('AAPL');
    expect(callFmp).toHaveBeenCalledWith('fmp_search_name', { query: 'Apple', limit: 5 });
  });

  it('returns first result if no name match on major exchange', async () => {
    const callFmp = vi.fn().mockResolvedValue([
      { symbol: 'XYZ', name: 'Xyz Corp', exchangeShortName: 'OTC' },
    ]);
    const result = await resolveSymbol('Unknown Corp', callFmp);
    expect(result).toBe('XYZ');
  });

  it('returns null when search returns empty array', async () => {
    const callFmp = vi.fn().mockResolvedValue([]);
    const result = await resolveSymbol('Nonexistent', callFmp);
    expect(result).toBeNull();
  });

  it('returns null when search throws', async () => {
    const callFmp = vi.fn().mockRejectedValue(new Error('Network error'));
    const result = await resolveSymbol('Apple', callFmp);
    expect(result).toBeNull();
  });

  it('returns null when search returns non-array', async () => {
    const callFmp = vi.fn().mockResolvedValue(null);
    const result = await resolveSymbol('Apple', callFmp);
    expect(result).toBeNull();
  });
});

// ─── fetchMarketData ────────────────────────────────────────────────────────

describe('fetchMarketData', () => {
  it('calls 6 FMP tools in parallel and returns raw data', async () => {
    const callFmp = vi.fn().mockImplementation((tool: string) => {
      const data: Record<string, unknown> = {
        fmp_income_statement: [{ revenue: 394e9, ebitda: 130e9 }],
        fmp_balance_sheet: [{ totalAssets: 352e9, totalDebt: 111e9 }],
        fmp_cash_flow: [{ operatingCashFlow: 110e9, capitalExpenditure: -11e9 }],
        fmp_key_metrics: [{ debtToEquity: 1.76, currentRatio: 0.99 }],
        fmp_company_profile: [{ beta: 1.28, mktCap: 2.8e12, companyName: 'Apple Inc.' }],
        fmp_quote: [{ price: 182.5, sharesOutstanding: 15.3e9 }],
      };
      return Promise.resolve(data[tool] ?? []);
    });

    const result = await fetchMarketData('AAPL', callFmp);

    expect(callFmp).toHaveBeenCalledTimes(6);
    expect(result.income).toEqual({ revenue: 394e9, ebitda: 130e9 });
    expect(result.balanceSheet).toEqual({ totalAssets: 352e9, totalDebt: 111e9 });
    expect(result.profile).toEqual({ beta: 1.28, mktCap: 2.8e12, companyName: 'Apple Inc.' });
  });

  it('handles individual tool failures gracefully', async () => {
    const callFmp = vi.fn().mockImplementation((tool: string) => {
      if (tool === 'fmp_balance_sheet') return Promise.reject(new Error('timeout'));
      return Promise.resolve([{ revenue: 1e9 }]);
    });

    const result = await fetchMarketData('AAPL', callFmp);
    expect(result.income).toBeDefined();
    expect(result.balanceSheet).toBeUndefined();
  });
});

// ─── mapFmpToMetrics ────────────────────────────────────────────────────────

describe('mapFmpToMetrics', () => {
  const sampleFmp: FmpRawData = {
    income: {
      revenue: 394e9,
      ebitda: 130e9,
      operatingIncome: 120e9,
      netIncome: 97e9,
      costOfRevenue: 223e9,
      eps: 6.13,
      interestExpense: 3.9e9,
      depreciationAndAmortization: 11e9,
    },
    balanceSheet: {
      totalAssets: 352e9,
      totalStockholdersEquity: 62e9,
      totalDebt: 111e9,
      netDebt: 81e9,
      cashAndCashEquivalents: 30e9,
      totalCurrentAssets: 135e9,
      totalCurrentLiabilities: 154e9,
      netReceivables: 60e9,
      inventory: 7e9,
      accountPayables: 62e9,
      propertyPlantEquipmentNet: 43e9,
    },
    cashFlow: {
      operatingCashFlow: 110e9,
      capitalExpenditure: -11e9,
    },
    keyMetrics: {
      debtToEquity: 1.79,
      currentRatio: 0.88,
      interestCoverage: 33.3,
      enterpriseValue: 2.85e12,
    },
    profile: {
      beta: 1.28,
      mktCap: 2.8e12,
      companyName: 'Apple Inc.',
      sector: 'Technology',
      industry: 'Consumer Electronics',
      lastDiv: 0.96,
    },
    quote: {
      price: 182.5,
      sharesOutstanding: 15.3e9,
    },
  };

  it('maps all income statement fields', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m.revenue).toBe(394e9);
    expect(m.ebitda).toBe(130e9);
    expect(m.ebit).toBe(120e9);
    expect(m.net_income).toBe(97e9);
    expect(m.cogs).toBe(223e9);
    expect(m.eps).toBe(6.13);
    expect(m.interest_expense).toBe(3.9e9);
    expect(m.depreciation).toBe(11e9);
  });

  it('maps all balance sheet fields', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m.total_assets).toBe(352e9);
    expect(m.total_equity).toBe(62e9);
    expect(m.total_debt).toBe(111e9);
    expect(m.net_debt).toBe(81e9);
    expect(m.cash).toBe(30e9);
    expect(m.current_assets).toBe(135e9);
    expect(m.current_liabilities).toBe(154e9);
    expect(m.receivables).toBe(60e9);
    expect(m.inventory).toBe(7e9);
    expect(m.payables).toBe(62e9);
    expect(m.ppe).toBe(43e9);
  });

  it('maps cash flow fields with abs(capex)', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m.operating_cash_flow).toBe(110e9);
    expect(m.capex).toBe(11e9); // abs of -11e9
  });

  it('maps key metrics and profile fields', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m.debt_to_equity).toBe(1.79);
    expect(m.current_ratio).toBe(0.88);
    expect(m.interest_coverage).toBe(33.3);
    expect(m.enterprise_value).toBe(2.85e12);
    expect(m.beta).toBe(1.28);
    expect(m.market_cap).toBe(2.8e12);
    expect(m.share_price).toBe(182.5);
    expect(m.shares_outstanding).toBe(15.3e9);
    expect(m.dividend_per_share).toBe(0.96);
  });

  it('computes derived margins', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m.ebitda_margin).toBeCloseTo(130e9 / 394e9, 4);
    expect(m.net_margin).toBeCloseTo(97e9 / 394e9, 4);
    expect(m.gross_margin).toBeCloseTo((394e9 - 223e9) / 394e9, 4);
  });

  it('sets symbol and company metadata', () => {
    const m = mapFmpToMetrics(sampleFmp, 'AAPL');
    expect(m._symbol).toBe('AAPL');
    expect(m._company).toBe('Apple Inc.');
    expect(m._sector).toBe('Technology');
    expect(m._industry).toBe('Consumer Electronics');
  });

  it('handles missing data gracefully', () => {
    const m = mapFmpToMetrics({}, 'AAPL');
    expect(m._symbol).toBe('AAPL');
    expect(m.revenue).toBeUndefined();
    expect(m.beta).toBeUndefined();
  });
});

// ─── mergeMetrics ───────────────────────────────────────────────────────────

describe('mergeMetrics', () => {
  it('FMP fills missing text metrics', () => {
    const text: ExtractedMetrics = { _raw: 'Analyze Apple', _company: 'Apple' };
    const fmp: Partial<ExtractedMetrics> = { revenue: 394e9, beta: 1.28, _symbol: 'AAPL' };
    const merged = mergeMetrics(text, fmp);
    expect(merged.revenue).toBe(394e9);
    expect(merged.beta).toBe(1.28);
    expect(merged._symbol).toBe('AAPL');
    expect(merged._dataSource).toBe('fmp-enriched');
  });

  it('text-parsed values override FMP values', () => {
    const text: ExtractedMetrics = { _raw: 'revenue 500B', _company: 'Apple', revenue: 500e9 };
    const fmp: Partial<ExtractedMetrics> = { revenue: 394e9, beta: 1.28 };
    const merged = mergeMetrics(text, fmp);
    expect(merged.revenue).toBe(500e9); // text wins
    expect(merged.beta).toBe(1.28);     // FMP fills gap
  });

  it('never overwrites _raw', () => {
    const text: ExtractedMetrics = { _raw: 'original query', _company: 'Apple' };
    const fmp: Partial<ExtractedMetrics> = { _raw: 'should not overwrite' } as any;
    const merged = mergeMetrics(text, fmp);
    expect(merged._raw).toBe('original query');
  });

  it('always sets metadata fields from FMP', () => {
    const text: ExtractedMetrics = { _raw: 'test', _company: 'Old Name' };
    const fmp: Partial<ExtractedMetrics> = { _symbol: 'AAPL', _sector: 'Tech', _industry: 'Electronics', _company: 'Apple Inc.' };
    const merged = mergeMetrics(text, fmp);
    expect(merged._symbol).toBe('AAPL');
    expect(merged._sector).toBe('Tech');
    expect(merged._industry).toBe('Electronics');
  });
});

// ─── enrichMetrics ──────────────────────────────────────────────────────────

describe('enrichMetrics', () => {
  it('enriches metrics with live FMP data', async () => {
    const callFmp = vi.fn().mockImplementation((tool: string) => {
      if (tool === 'fmp_search_name') {
        return Promise.resolve([{ symbol: 'AAPL', name: 'Apple Inc.', exchangeShortName: 'NASDAQ' }]);
      }
      return Promise.resolve([{ revenue: 394e9, totalAssets: 352e9, beta: 1.28, price: 182.5, companyName: 'Apple Inc.' }]);
    });

    const text: ExtractedMetrics = { _raw: 'Analyze Apple credit risk', _company: 'Apple' };
    const result = await enrichMetrics(text, callFmp);

    expect(result._dataSource).toBe('fmp-enriched');
    expect(result.revenue).toBe(394e9);
  });

  it('falls back to text-only when no company name', async () => {
    const callFmp = vi.fn();
    const text: ExtractedMetrics = { _raw: 'Run a DCF with revenue 1B' };
    const result = await enrichMetrics(text, callFmp);

    expect(result._dataSource).toBe('text-only');
    expect(callFmp).not.toHaveBeenCalled();
  });

  it('falls back gracefully when FMP errors', async () => {
    const callFmp = vi.fn().mockRejectedValue(new Error('API down'));
    const text: ExtractedMetrics = { _raw: 'Analyze Apple', _company: 'Apple' };
    const result = await enrichMetrics(text, callFmp);

    expect(result._dataSource).toBe('text-only');
    expect(result._company).toBe('Apple');
  });

  it('falls back when symbol resolution fails', async () => {
    const callFmp = vi.fn().mockResolvedValue([]);
    const text: ExtractedMetrics = { _raw: 'Analyze FooBar Corp', _company: 'FooBar Corp' };
    const result = await enrichMetrics(text, callFmp);

    expect(result._dataSource).toBe('text-only');
  });
});
