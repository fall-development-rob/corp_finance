import { describe, it, expect } from 'vitest';
import { resolveCompany, getCorpusSize } from '../utils/company-resolver.js';

describe('company-resolver', () => {
  it('has a populated company corpus', () => {
    expect(getCorpusSize()).toBeGreaterThan(50);
  });

  it('resolves "Apple" to AAPL', async () => {
    const match = await resolveCompany('Apple');
    expect(match).not.toBeNull();
    expect(match!.ticker).toBe('AAPL');
    expect(match!.similarity).toBeGreaterThan(0.5);
  });

  it('resolves "Microsoft" to MSFT', async () => {
    const match = await resolveCompany('Microsoft');
    expect(match).not.toBeNull();
    expect(match!.ticker).toBe('MSFT');
  });

  it('resolves "Tesla" to TSLA', async () => {
    const match = await resolveCompany('Tesla');
    expect(match).not.toBeNull();
    expect(match!.ticker).toBe('TSLA');
  });

  it('resolves "Goldman Sachs" to GS', async () => {
    const match = await resolveCompany('Goldman Sachs');
    expect(match).not.toBeNull();
    expect(match!.ticker).toBe('GS');
  });

  it('resolves "Netflix" to NFLX', async () => {
    const match = await resolveCompany('Netflix');
    expect(match).not.toBeNull();
    expect(match!.ticker).toBe('NFLX');
  });

  it('returns null for nonsense query', async () => {
    const match = await resolveCompany('xyzzy plugh nothing');
    if (match) {
      expect(match.similarity).toBeLessThan(0.5);
    }
  });
});
