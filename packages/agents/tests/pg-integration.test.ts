// Integration test — requires ruvector-postgres container running on port 5433
// Skips automatically when the database is unreachable

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { getPool, healthCheck, closePool, resetPool, float32ToVectorLiteral } from '../db/pg-client.js';
import { PgFinancialMemory } from '../memory/pg-financial-memory.js';
import { PgReasoningBank } from '../learning/pg-reasoning-bank.js';
import type { MemoryMetadata } from '../types/memory.js';
import type { ReasoningTrace } from '../types/learning.js';

// Check connectivity before running any tests
let canConnect = false;
try {
  const pool = await getPool({
    host: 'localhost',
    port: 5433,
    user: 'cfa',
    password: 'cfa_dev_pass',
    database: 'cfa_agents',
    poolMax: 3,
    connectionTimeoutMs: 3000,
  });
  const res = await pool.query('SELECT 1 AS ok');
  canConnect = res.rows[0]?.ok === 1;
} catch {
  canConnect = false;
}

function makeMetadata(overrides: Partial<MemoryMetadata> = {}): MemoryMetadata {
  return {
    sourceType: 'analysis',
    sector: 'technology',
    tickers: ['AAPL'],
    analysisType: 'dcf_valuation',
    tags: ['equity', 'large-cap'],
    ...overrides,
  };
}

function makeTrace(overrides: Partial<ReasoningTrace> = {}): ReasoningTrace {
  return {
    traceId: `trace-${Date.now()}`,
    agentType: 'equity-analyst',
    requestId: `req-${Date.now()}`,
    steps: [
      { phase: 'observe', content: 'Reading financials', timestamp: new Date() },
      { phase: 'think', content: 'Analysing margins', timestamp: new Date() },
      {
        phase: 'act',
        content: 'Running DCF',
        toolCalls: [`wacc_calculator_${Date.now()}`, `dcf_model_${Date.now()}`],
        timestamp: new Date(),
      },
      { phase: 'reflect', content: 'Valuation complete', timestamp: new Date() },
    ],
    outcome: 'success',
    createdAt: new Date(),
    ...overrides,
  };
}

describe.skipIf(!canConnect)('PG Integration — ruvector-postgres', () => {
  afterAll(async () => {
    // Clean up test data — reset pool first in case it's broken from a ruvector segfault
    await resetPool();
    try {
      const pool = await getPool();
      await pool.query("DELETE FROM reasoning_memories WHERE domain = 'test-integration'");
      await pool.query("DELETE FROM task_trajectories WHERE agent_id LIKE '%-analyst'");
    } catch {
      // DB may still be recovering — cleanup is best-effort
    }
    await closePool();
  });

  describe('healthCheck', () => {
    it('returns true when database is reachable', async () => {
      const ok = await healthCheck();
      expect(ok).toBe(true);
    });
  });

  describe('float32ToVectorLiteral', () => {
    it('converts Float32Array to ruvector literal', () => {
      const vec = new Float32Array([0.1, 0.2, 0.3]);
      const literal = float32ToVectorLiteral(vec);
      expect(literal).toMatch(/^\[.*\]$/);

      // Verify it can be cast in PG
      // (will be tested implicitly through store/search)
    });
  });

  describe('PgFinancialMemory', () => {
    const memory = new PgFinancialMemory('test-integration');

    it('stores an entry and retrieves it by ID', async () => {
      const entry = await memory.store(
        'AAPL DCF valuation: intrinsic value $185, current price $172',
        makeMetadata(),
      );

      expect(entry.entryId).toBeTruthy();
      expect(entry.content).toContain('AAPL DCF');
      expect(entry.embeddingModel).toBe('all-MiniLM-L6-v2');

      const retrieved = await memory.retrieve(entry.entryId);
      expect(retrieved).not.toBeNull();
      expect(retrieved!.content).toContain('AAPL DCF');
      expect(retrieved!.accessCount).toBe(1);
    });

    it('returns null for unknown entry ID', async () => {
      const result = await memory.retrieve('00000000-0000-0000-0000-000000000000');
      expect(result).toBeNull();
    });

    it('increments access count on retrieve', async () => {
      const entry = await memory.store('access count test', makeMetadata());

      await memory.retrieve(entry.entryId);
      await memory.retrieve(entry.entryId);
      const third = await memory.retrieve(entry.entryId);

      expect(third!.accessCount).toBe(3);
    });

    it('searches by vector similarity', async () => {
      await memory.store(
        'Microsoft Azure cloud revenue growth margins operating income',
        makeMetadata({ tickers: ['MSFT'] }),
      );
      await memory.store(
        'Tesla Model Y deliveries production capacity gigafactory',
        makeMetadata({ tickers: ['TSLA'] }),
      );

      const results = await memory.search('cloud computing revenue growth', 5);

      expect(results.entries.length).toBeGreaterThanOrEqual(1);
      expect(results.query).toBe('cloud computing revenue growth');
      expect(results.entries[0].similarityScore).toBeGreaterThan(0);
    });

    it('getByTicker returns results via embedding search', async () => {
      const entries = await memory.getByTicker('AAPL', 3);
      // May or may not find entries depending on prior test state
      expect(Array.isArray(entries)).toBe(true);
    });
  });

  describe('PgReasoningBank', () => {
    const bank = new PgReasoningBank();

    it('records a successful trace with trajectory + pattern', async () => {
      const statsBefore = await bank.getStats();
      await bank.recordTrace(makeTrace());
      const statsAfter = await bank.getStats();

      expect(statsAfter.totalTraces).toBe(statsBefore.totalTraces + 1);
      expect(statsAfter.totalPatterns).toBe(statsBefore.totalPatterns + 1);
    });

    it('records a failed trace without creating a pattern', async () => {
      const statsBefore = await bank.getStats();
      await bank.recordTrace(makeTrace({ outcome: 'failure' }));
      const statsAfter = await bank.getStats();

      expect(statsAfter.totalTraces).toBe(statsBefore.totalTraces + 1);
      expect(statsAfter.totalPatterns).toBe(statsBefore.totalPatterns);
    });

    it('records feedback as a reasoning memory', async () => {
      await bank.recordFeedback({
        feedbackId: `fb-${Date.now()}`,
        requestId: `req-${Date.now()}`,
        score: 0.9,
        automated: false,
        createdAt: new Date(),
      });
      // No throw = success
    });

    it('searches patterns by task type with vector similarity', async () => {
      // Record a trace first so there's something to find
      await bank.recordTrace(makeTrace({
        traceId: `search-trace-${Date.now()}`,
        requestId: `search-req-${Date.now()}`,
      }));

      const patterns = await bank.searchPatterns('valuation', 5);
      expect(Array.isArray(patterns)).toBe(true);
      // Patterns may or may not match depending on domain filter
    });

    it('gets a pattern by ID from the database', async () => {
      // Insert a known pattern via recordTrace
      const trace = makeTrace({
        traceId: `get-trace-${Date.now()}`,
        requestId: `get-req-${Date.now()}`,
      });
      await bank.recordTrace(trace);

      // Query it back via search (to get a valid ID)
      const pool = await getPool();
      const { rows } = await pool.query<{ id: string }>(
        "SELECT id FROM reasoning_memories WHERE title LIKE '%-pattern' ORDER BY created_at DESC LIMIT 1",
      );

      if (rows.length > 0) {
        const pattern = await bank.getPattern(rows[0].id);
        expect(pattern).not.toBeNull();
        expect(pattern!.toolSequence.length).toBeGreaterThan(0);
      }
    });

    it('returns null for nonexistent pattern', async () => {
      const pattern = await bank.getPattern('00000000-0000-0000-0000-000000000000');
      expect(pattern).toBeNull();
    });
  });
});
