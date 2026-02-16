import { describe, it, expect, vi } from 'vitest';
import { ExpertRouter } from '../config/expert-router.js';
import { ChiefAnalyst } from '../agents/chief-analyst.js';
import { SimpleEventBus } from '../types/events.js';

describe('ExpertRouter', () => {
  describe('initialization', () => {
    it('is not available before first route()', () => {
      const router = new ExpertRouter();
      expect(router.isAvailable).toBe(false);
    });

    it('initializes lazily on first route() call', async () => {
      const router = new ExpertRouter();
      const results = await router.route('test query');
      // In CI this typically returns [] (hash-based embeddings)
      // but should NOT throw
      expect(Array.isArray(results)).toBe(true);
    });

    it('returns empty array when initialization fails', async () => {
      const router = new ExpertRouter();
      (router as any).initFailed = true;
      const results = await router.route('equity valuation DCF');
      expect(results).toEqual([]);
    });

    it('can reset and re-initialize', () => {
      const router = new ExpertRouter();
      (router as any).initFailed = true;
      router.reset();
      expect(router.isAvailable).toBe(false);
      expect((router as any).initFailed).toBe(false);
    });
  });

  describe('route() with mocked router', () => {
    it('returns ranked agents for equity query', async () => {
      const router = new ExpertRouter({ threshold: 0.1 });
      (router as any).router = {
        route: vi.fn().mockResolvedValue([
          { intent: 'equity-analyst', score: 0.92 },
          { intent: 'quant-risk-analyst', score: 0.61 },
        ]),
      };

      const results = await router.route('run a DCF valuation');
      expect(results).toHaveLength(2);
      expect(results[0].agentType).toBe('equity-analyst');
      expect(results[0].score).toBe(0.92);
      expect(results[1].agentType).toBe('quant-risk-analyst');
    });

    it('passes through results from underlying router (threshold applied in constructor)', async () => {
      const router = new ExpertRouter({ threshold: 0.8 });
      // SemanticRouter applies threshold internally; our route() passes results through
      (router as any).router = {
        route: vi.fn().mockResolvedValue([
          { intent: 'equity-analyst', score: 0.3 },
        ]),
      };

      const results = await router.route('something vague');
      expect(results).toHaveLength(1);
      expect(results[0].agentType).toBe('equity-analyst');
    });

    it('respects k parameter', async () => {
      const router = new ExpertRouter({ threshold: 0.1 });
      (router as any).router = {
        route: vi.fn().mockResolvedValue([
          { intent: 'credit-analyst', score: 0.9 },
        ]),
      };

      await router.route('credit analysis', 1);
      expect((router as any).router.route).toHaveBeenCalledWith('credit analysis', 1);
    });

    it('handles route() errors gracefully', async () => {
      const router = new ExpertRouter({ threshold: 0.1 });
      (router as any).router = {
        route: vi.fn().mockRejectedValue(new Error('boom')),
      };

      const results = await router.route('anything');
      expect(results).toEqual([]);
    });
  });

  describe('routeStep()', () => {
    it('returns best single agent', async () => {
      const router = new ExpertRouter({ threshold: 0.1 });
      (router as any).router = {
        route: vi.fn().mockResolvedValue([
          { intent: 'credit-analyst', score: 0.88 },
        ]),
      };

      const result = await router.routeStep('Assess credit quality');
      expect(result).not.toBeNull();
      expect(result!.agentType).toBe('credit-analyst');
      expect(result!.score).toBe(0.88);
    });

    it('returns null when unavailable', async () => {
      const router = new ExpertRouter();
      (router as any).initFailed = true;

      const result = await router.routeStep('something');
      expect(result).toBeNull();
    });
  });
});

describe('ChiefAnalyst — semantic routing integration', () => {
  function createChief(expertRouter?: ExpertRouter) {
    return new ChiefAnalyst({
      confidenceThreshold: 0.6,
      maxSpecialists: 6,
      eventBus: new SimpleEventBus(),
      expertRouter,
    });
  }

  it('routeQuery uses semantic routing when available', async () => {
    const mockRouter = new ExpertRouter();
    (mockRouter as any).router = {
      route: vi.fn().mockResolvedValue([
        { intent: 'equity-analyst', score: 0.9 },
        { intent: 'quant-risk-analyst', score: 0.7 },
      ]),
    };

    const chief = createChief(mockRouter);
    const { intent, agents } = await chief.routeQuery('DCF valuation for Apple');

    expect(intent.type).toBe('valuation');
    expect(agents).toHaveLength(2);
    expect(agents[0].agentType).toBe('equity-analyst');
    expect(agents[0].score).toBe(0.9);
  });

  it('routeQuery falls back to static when semantic returns empty', async () => {
    const mockRouter = new ExpertRouter();
    (mockRouter as any).router = {
      route: vi.fn().mockResolvedValue([]),
    };

    const chief = createChief(mockRouter);
    const { intent, agents } = await chief.routeQuery('What is the credit rating?');

    // Static classifyIntent detects "credit" keyword
    expect(intent.domains).toContain('credit');
    expect(agents.length).toBeGreaterThan(0);
    expect(agents[0].score).toBe(0); // score 0 = static fallback
  });

  it('routeQuery works without expertRouter (backward compat)', async () => {
    const chief = createChief(); // no router
    const { intent, agents } = await chief.routeQuery('equity valuation');

    expect(intent.type).toBe('valuation');
    expect(agents.length).toBeGreaterThan(0);
  });

  it('createPlan uses routedAgents when provided', () => {
    const chief = createChief();
    const request = chief.createRequest('test query');
    const routedAgents = [
      { agentType: 'equity-analyst', score: 0.9 },
      { agentType: 'credit-analyst', score: 0.7 },
    ];

    const plan = chief.createPlan(request, routedAgents);

    // Plan should have equity + credit steps (+ synthesis)
    const nonSynthesisSteps = plan.steps.filter(s => s.id !== 'step-synthesis');
    expect(nonSynthesisSteps).toHaveLength(2);

    // First step should reference equity-analyst tools
    expect(nonSynthesisSteps[0].description).toContain('Equity');
  });

  it('createPlan falls back to suggestAgents when no routedAgents', () => {
    const chief = createChief();
    const request = chief.createRequest('credit analysis');

    const plan = chief.createPlan(request);
    const nonSynthesisSteps = plan.steps.filter(s => s.id !== 'step-synthesis');
    expect(nonSynthesisSteps.length).toBeGreaterThan(0);
  });
});
