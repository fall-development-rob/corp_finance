import { describe, it, expect } from 'vitest';
import { SymbolSchema, SymbolPeriodSchema, SearchSchema, ScreenerSchema } from '../src/schemas/common.js';
import { QuoteSchema, BatchQuoteSchema, HistoricalPriceSchema, IntradaySchema } from '../src/schemas/quotes.js';
import { EarningsTranscriptSchema, AnalystEstimatesSchema } from '../src/schemas/earnings.js';
import { IndexConstituentsSchema, EconomicIndicatorSchema } from '../src/schemas/market.js';

describe('Schema Validation', () => {
  describe('SymbolSchema', () => {
    it('accepts valid symbol', () => {
      expect(SymbolSchema.parse({ symbol: 'AAPL' })).toEqual({ symbol: 'AAPL' });
    });

    it('rejects empty symbol', () => {
      expect(() => SymbolSchema.parse({ symbol: '' })).toThrow();
    });

    it('rejects missing symbol', () => {
      expect(() => SymbolSchema.parse({})).toThrow();
    });
  });

  describe('SymbolPeriodSchema', () => {
    it('accepts symbol with defaults', () => {
      const result = SymbolPeriodSchema.parse({ symbol: 'MSFT' });
      expect(result.symbol).toBe('MSFT');
      expect(result.period).toBe('annual');
      expect(result.limit).toBe(4);
    });

    it('accepts explicit period and limit', () => {
      const result = SymbolPeriodSchema.parse({ symbol: 'GOOG', period: 'quarter', limit: 8 });
      expect(result.period).toBe('quarter');
      expect(result.limit).toBe(8);
    });

    it('rejects invalid period', () => {
      expect(() => SymbolPeriodSchema.parse({ symbol: 'AAPL', period: 'monthly' })).toThrow();
    });

    it('rejects limit > 120', () => {
      expect(() => SymbolPeriodSchema.parse({ symbol: 'AAPL', limit: 200 })).toThrow();
    });
  });

  describe('SearchSchema', () => {
    it('accepts query with defaults', () => {
      const result = SearchSchema.parse({ query: 'Apple' });
      expect(result.query).toBe('Apple');
      expect(result.limit).toBe(10);
    });

    it('rejects empty query', () => {
      expect(() => SearchSchema.parse({ query: '' })).toThrow();
    });
  });

  describe('ScreenerSchema', () => {
    it('accepts empty screener (all defaults)', () => {
      const result = ScreenerSchema.parse({});
      expect(result.limit).toBe(50);
    });

    it('accepts full screener params', () => {
      const result = ScreenerSchema.parse({
        market_cap_more_than: 1e9,
        market_cap_less_than: 1e12,
        sector: 'Technology',
        industry: 'Software',
        exchange: 'NASDAQ',
        country: 'US',
        limit: 100,
      });
      expect(result.sector).toBe('Technology');
    });
  });

  describe('QuoteSchema', () => {
    it('is the same as SymbolSchema', () => {
      const result = QuoteSchema.parse({ symbol: 'TSLA' });
      expect(result.symbol).toBe('TSLA');
    });

    it('rejects missing symbol', () => {
      expect(() => QuoteSchema.parse({})).toThrow();
    });
  });

  describe('BatchQuoteSchema', () => {
    it('accepts comma-separated symbols', () => {
      const result = BatchQuoteSchema.parse({ symbols: 'AAPL,MSFT,GOOGL' });
      expect(result.symbols).toBe('AAPL,MSFT,GOOGL');
    });

    it('rejects empty symbols', () => {
      expect(() => BatchQuoteSchema.parse({ symbols: '' })).toThrow();
    });
  });

  describe('HistoricalPriceSchema', () => {
    it('accepts symbol only', () => {
      const result = HistoricalPriceSchema.parse({ symbol: 'AAPL' });
      expect(result.symbol).toBe('AAPL');
      expect(result.from).toBeUndefined();
      expect(result.to).toBeUndefined();
    });

    it('accepts symbol with date range', () => {
      const result = HistoricalPriceSchema.parse({
        symbol: 'AAPL',
        from: '2023-01-01',
        to: '2024-01-01',
      });
      expect(result.from).toBe('2023-01-01');
      expect(result.to).toBe('2024-01-01');
    });
  });

  describe('IntradaySchema', () => {
    it('defaults to 5min interval', () => {
      const result = IntradaySchema.parse({ symbol: 'AAPL' });
      expect(result.interval).toBe('5min');
    });

    it('accepts all valid intervals', () => {
      for (const interval of ['1min', '5min', '15min', '30min', '1hour', '4hour']) {
        expect(() => IntradaySchema.parse({ symbol: 'AAPL', interval })).not.toThrow();
      }
    });

    it('rejects invalid interval', () => {
      expect(() => IntradaySchema.parse({ symbol: 'AAPL', interval: '2min' })).toThrow();
    });
  });

  describe('EarningsTranscriptSchema', () => {
    it('accepts valid year and quarter', () => {
      const result = EarningsTranscriptSchema.parse({ symbol: 'AAPL', year: 2024, quarter: 3 });
      expect(result.year).toBe(2024);
      expect(result.quarter).toBe(3);
    });

    it('rejects quarter > 4', () => {
      expect(() => EarningsTranscriptSchema.parse({ symbol: 'AAPL', year: 2024, quarter: 5 })).toThrow();
    });

    it('rejects quarter < 1', () => {
      expect(() => EarningsTranscriptSchema.parse({ symbol: 'AAPL', year: 2024, quarter: 0 })).toThrow();
    });

    it('rejects year < 2000', () => {
      expect(() => EarningsTranscriptSchema.parse({ symbol: 'AAPL', year: 1999, quarter: 1 })).toThrow();
    });
  });

  describe('AnalystEstimatesSchema', () => {
    it('accepts symbol with defaults', () => {
      const result = AnalystEstimatesSchema.parse({ symbol: 'AAPL' });
      expect(result.period).toBe('annual');
      expect(result.limit).toBe(4);
    });

    it('accepts quarterly period', () => {
      const result = AnalystEstimatesSchema.parse({ symbol: 'AAPL', period: 'quarter', limit: 8 });
      expect(result.period).toBe('quarter');
      expect(result.limit).toBe(8);
    });

    it('rejects limit > 30', () => {
      expect(() => AnalystEstimatesSchema.parse({ symbol: 'AAPL', limit: 50 })).toThrow();
    });
  });

  describe('IndexConstituentsSchema', () => {
    it('defaults to sp500', () => {
      const result = IndexConstituentsSchema.parse({});
      expect(result.index).toBe('sp500');
    });

    it('accepts all valid indices', () => {
      for (const index of ['sp500', 'nasdaq', 'dowjones']) {
        expect(() => IndexConstituentsSchema.parse({ index })).not.toThrow();
      }
    });

    it('rejects invalid index', () => {
      expect(() => IndexConstituentsSchema.parse({ index: 'ftse100' })).toThrow();
    });
  });

  describe('EconomicIndicatorSchema', () => {
    it('accepts indicator name', () => {
      const result = EconomicIndicatorSchema.parse({ name: 'GDP' });
      expect(result.name).toBe('GDP');
    });

    it('accepts with date range', () => {
      const result = EconomicIndicatorSchema.parse({ name: 'CPI', from: '2023-01-01', to: '2024-01-01' });
      expect(result.from).toBe('2023-01-01');
      expect(result.to).toBe('2024-01-01');
    });

    it('rejects empty name', () => {
      expect(() => EconomicIndicatorSchema.parse({ name: '' })).toThrow();
    });
  });
});
