// CFA Multi-Agent Pipeline — 6-stage orchestration
//
// User Request → Task Router → Agents (dynamic) → Coordination (attention)
//   → Vector Search (HNSW+GNN) → Synthesis (consensus) → Response

import { randomUUID } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import type { AgentDefinition } from 'agentic-flow/dist/utils/agentLoader.js';
import type { AgentIntent, MultiIntentResult } from 'agentic-flow/dist/routing/SemanticRouter.js';
import type { AgentOutput, SwarmTopology, CoordinationResult } from 'agentic-flow/dist/coordination/attention-coordinator.js';
import type { ReasoningBank } from '../learning/reasoning-bank.js';
import type { FinancialMemory } from '../memory/financial-memory.js';
import type { TaskType } from '../types/learning.js';

// ── Re-export topology type ─────────────────────────────────────────

export type Topology = SwarmTopology;

// ── Pipeline types ──────────────────────────────────────────────────

export interface PipelineConfig {
  topology: Topology;
  confidenceThreshold: number;
  maxAgents: number;
  attentionMechanism: 'flash' | 'multi-head';
  enableLearning: boolean;
  onStatus?: (stage: string, message: string) => void;
}

export interface AgentResult {
  agentName: string;
  agentType: string;
  output: string;
  attentionWeight: number;
}

export interface PipelineResult {
  requestId: string;
  synthesis: string;
  agentResults: AgentResult[];
  routedAgents: string[];
  topology: Topology;
  coordination?: {
    mechanism: string;
    executionTimeMs: number;
    topAgents: string[];
    attentionWeights: number[];
  };
  timings: {
    routingMs: number;
    memoryMs: number;
    agentsMs: number;
    coordinationMs: number;
    synthesisMs: number;
    totalMs: number;
  };
}

export class PipelineError extends Error {
  constructor(
    message: string,
    public readonly stage: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = 'PipelineError';
  }
}

// ── Directories ─────────────────────────────────────────────────────

const __pipelineDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__pipelineDir, '..', '..', '..', '..');
const cfaAgentsDir = join(repoRoot, '.claude', 'agents', 'cfa');
const skillsDir = join(repoRoot, '.claude', 'skills');

// ── Skill injection (shared with cli.ts) ────────────────────────────

export const AGENT_SKILLS: Record<string, string[]> = {
  'cfa-chief-analyst': [
    'corp-finance-tools-core',
    'corp-finance-tools-markets',
    'corp-finance-tools-risk',
    'corp-finance-tools-regulatory',
    'fmp-market-data',
    'fmp-research',
  ],
  'cfa-equity-analyst':          ['corp-finance-tools-core', 'fmp-market-data'],
  'cfa-credit-analyst':          ['corp-finance-tools-core', 'fmp-market-data'],
  'cfa-private-markets-analyst': ['corp-finance-tools-core', 'fmp-market-data'],
  'cfa-fixed-income-analyst':    ['corp-finance-tools-markets', 'fmp-market-data'],
  'cfa-derivatives-analyst':     ['corp-finance-tools-markets', 'fmp-market-data'],
  'cfa-macro-analyst':           ['corp-finance-tools-markets', 'fmp-research'],
  'cfa-quant-risk-analyst':      ['corp-finance-tools-risk', 'fmp-market-data'],
  'cfa-esg-regulatory-analyst':  ['corp-finance-tools-regulatory', 'fmp-research'],
};

const skillCache = new Map<string, string>();

function readSkillBody(skillName: string): string {
  if (skillCache.has(skillName)) return skillCache.get(skillName)!;

  const skillPath = join(skillsDir, skillName, 'SKILL.md');
  if (!existsSync(skillPath)) return '';

  const raw = readFileSync(skillPath, 'utf-8');
  const body = raw.replace(/^---\n[\s\S]*?\n---\n/, '').trim();
  skillCache.set(skillName, body);
  return body;
}

export function injectSkills(agent: AgentDefinition): AgentDefinition {
  const skills = AGENT_SKILLS[agent.name];
  if (!skills || skills.length === 0) return agent;

  const skillContent = skills
    .map(readSkillBody)
    .filter(Boolean)
    .join('\n\n---\n\n');

  if (!skillContent) return agent;

  return {
    ...agent,
    systemPrompt: agent.systemPrompt + '\n\n---\n\n# MCP Tool Reference\n\n' + skillContent,
  };
}

// ── CFA agent intent definitions for HNSW routing ──────────────────

const CFA_INTENTS: AgentIntent[] = [
  {
    agentType: 'cfa-chief-analyst',
    description: 'Research coordination, query decomposition, specialist delegation, result aggregation, quality gating',
    examples: [
      'Give me a comprehensive analysis of this company',
      'Prepare an institutional research report',
      'What are the key risks and opportunities here?',
    ],
    tags: ['coordination', 'research', 'aggregation', 'report', 'comprehensive', 'delegation', 'quality'],
  },
  {
    agentType: 'cfa-equity-analyst',
    description: 'Equity valuation, DCF, trading comps, earnings quality, dividend policy, financial forensics, target price',
    examples: [
      'Calculate WACC for beta 1.2, risk-free rate 4%',
      'Run a DCF model for revenue $500M growing at 8%',
      'What is the Beneish M-Score for these financials?',
      'Derive a target price using PE and DDM methods',
    ],
    tags: ['equity', 'valuation', 'dcf', 'wacc', 'comps', 'earnings', 'dividend', 'target-price', 'forensics', 'pe-ratio'],
  },
  {
    agentType: 'cfa-credit-analyst',
    description: 'Credit ratings, spreads, default probability, covenants, restructuring, debt capacity, Altman Z-score',
    examples: [
      'Assess credit quality: D/E 0.6, interest coverage 5x',
      'Calculate Altman Z-score for these metrics',
      'Evaluate covenant compliance for this debt structure',
      'What is the probability of default given these financials?',
    ],
    tags: ['credit', 'ratings', 'spreads', 'default', 'covenants', 'restructuring', 'debt', 'z-score', 'leverage'],
  },
  {
    agentType: 'cfa-fixed-income-analyst',
    description: 'Bond pricing, yield curves, duration/convexity, MBS analytics, municipal bonds, sovereign debt, repo financing',
    examples: [
      'Price this 10-year bond with 5% coupon at par',
      'Bootstrap the yield curve from these swap rates',
      'Calculate duration and convexity for this portfolio',
      'Analyze prepayment risk for this MBS tranche',
    ],
    tags: ['bonds', 'yield', 'duration', 'convexity', 'mbs', 'municipal', 'sovereign', 'repo', 'fixed-income', 'rates'],
  },
  {
    agentType: 'cfa-derivatives-analyst',
    description: 'Options pricing, implied volatility, vol surfaces, convertibles, structured products, real options, Greeks',
    examples: [
      'Price this call option using Black-Scholes',
      'Build a volatility surface from these market quotes',
      'Value this convertible bond with credit spread 200bps',
      'Calculate Greeks for this options portfolio',
    ],
    tags: ['options', 'volatility', 'greeks', 'derivatives', 'convertibles', 'structured', 'black-scholes', 'swaps'],
  },
  {
    agentType: 'cfa-quant-risk-analyst',
    description: 'VaR, factor models, portfolio optimization, risk budgeting, stress testing, market microstructure',
    examples: [
      'Calculate 99% VaR for this portfolio',
      'Run Markowitz mean-variance optimization',
      'Decompose risk by factor using Barra model',
      'Stress test portfolio against 2008 scenario',
    ],
    tags: ['var', 'risk', 'portfolio', 'optimization', 'factors', 'stress-test', 'sharpe', 'quant', 'microstructure'],
  },
  {
    agentType: 'cfa-macro-analyst',
    description: 'Interest rates, FX, commodities, emerging markets, trade finance, sovereign analysis, inflation',
    examples: [
      'What is the macro outlook for emerging markets?',
      'Analyze FX carry trade for USD/JPY',
      'Forecast commodity prices given supply constraints',
      'Evaluate sovereign credit risk for Brazil',
    ],
    tags: ['macro', 'rates', 'fx', 'commodities', 'emerging-markets', 'inflation', 'sovereign', 'gdp', 'trade'],
  },
  {
    agentType: 'cfa-esg-regulatory-analyst',
    description: 'ESG scores, carbon markets, compliance, AML/KYC, FATCA/CRS, tax treaties, transfer pricing, regulatory reporting',
    examples: [
      'Calculate ESG composite score for this company',
      'Assess carbon exposure and transition risk',
      'Check FATCA compliance for this structure',
      'Evaluate transfer pricing for intercompany transactions',
    ],
    tags: ['esg', 'carbon', 'compliance', 'aml', 'fatca', 'regulatory', 'tax', 'transfer-pricing', 'sustainability'],
  },
  {
    agentType: 'cfa-private-markets-analyst',
    description: 'PE/LBO models, M&A, venture, private credit, CLO/securitization, infrastructure, fund-of-funds, waterfall',
    examples: [
      'Build an LBO model with 4x leverage and 5-year hold',
      'Calculate IRR and MOIC for this PE deal',
      'Analyze CLO waterfall with these tranche specs',
      'Value this venture deal at Series B',
    ],
    tags: ['pe', 'lbo', 'ma', 'venture', 'private-credit', 'clo', 'infrastructure', 'irr', 'waterfall', 'fund'],
  },
];

// ── Agent name normalization ────────────────────────────────────────

function agentNameFromType(agentType: string): string {
  // CFA intents already use the full name like 'cfa-equity-analyst'
  return agentType;
}

function inferTaskType(agentType: string): TaskType {
  if (agentType.includes('equity')) return 'valuation';
  if (agentType.includes('credit')) return 'credit_assessment';
  if (agentType.includes('risk') || agentType.includes('quant')) return 'risk_analysis';
  if (agentType.includes('macro')) return 'macro_research';
  if (agentType.includes('esg') || agentType.includes('regulatory')) return 'esg_review';
  if (agentType.includes('private') || agentType.includes('pe')) return 'deal_analysis';
  if (agentType.includes('fixed') || agentType.includes('income')) return 'portfolio_construction';
  if (agentType.includes('derivatives')) return 'risk_analysis';
  return 'valuation';
}

// ── Embedder adapter ────────────────────────────────────────────────
// SemanticRouter's cosineSimilarity iterates embed() results as number[],
// but EmbeddingService.embed() returns { embedding: number[], latency }.
// This adapter makes embed() return the raw array so HNSW similarity works.

function createRouterEmbedder(realEmbedder: any): any {
  return {
    async embed(text: string) {
      const result = await realEmbedder.embed(text);
      return result.embedding;
    },
    async embedBatch(texts: string[]) {
      const results = await realEmbedder.embedBatch(texts);
      return results.map((r: any) => r.embedding);
    },
  };
}

// ── Default config ──────────────────────────────────────────────────

const DEFAULT_CONFIG: PipelineConfig = {
  topology: 'hierarchical',
  confidenceThreshold: 0.4,
  maxAgents: 6,
  attentionMechanism: 'flash',
  enableLearning: true,
};

const AGENT_TIMEOUT_MS = 120_000;
const DEFAULT_AGENT = 'cfa-chief-analyst';

// ── Pipeline class ──────────────────────────────────────────────────

export class CfaPipeline {
  private config: PipelineConfig;
  private initialized = false;

  // agentic-flow modules (loaded lazily)
  private claudeAgent!: typeof import('agentic-flow/dist/agents/claudeAgent.js')['claudeAgent'];
  private getAgent!: typeof import('agentic-flow/dist/utils/agentLoader.js')['getAgent'];

  // Routing & coordination (may be null if init fails gracefully)
  private semanticRouter: InstanceType<typeof import('agentic-flow/dist/routing/SemanticRouter.js')['SemanticRouter']> | null = null;
  private coordinator: InstanceType<typeof import('agentic-flow/dist/coordination/attention-coordinator.js')['AttentionCoordinator']> | null = null;
  private embedder: InstanceType<typeof import('agentic-flow/dist/core/embedding-service.js')['EmbeddingService']> | null = null;

  // Learning & memory (best-effort)
  private reasoningBank: ReasoningBank | null = null;
  private financialMemory: FinancialMemory | null = null;

  constructor(config?: Partial<PipelineConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  // ── Lazy initialization ───────────────────────────────────────────

  private async init(): Promise<void> {
    if (this.initialized) return;

    const _require = createRequire(import.meta.url);
    const _afDir = dirname(_require.resolve('agentic-flow/package.json'));
    const afImport = (subpath: string) =>
      import(pathToFileURL(join(_afDir, 'dist', ...subpath.split('/'))).href);

    // Core agent modules (required)
    const [agentMod, loaderMod] = await Promise.all([
      afImport('agents/claudeAgent.js') as Promise<typeof import('agentic-flow/dist/agents/claudeAgent.js')>,
      afImport('utils/agentLoader.js') as Promise<typeof import('agentic-flow/dist/utils/agentLoader.js')>,
    ]);
    this.claudeAgent = agentMod.claudeAgent;
    this.getAgent = loaderMod.getAgent;

    // Embedding service (Transformers.js local model — no API calls)
    try {
      const embMod = await afImport('core/embedding-service.js') as typeof import('agentic-flow/dist/core/embedding-service.js');
      const transformers = new embMod.TransformersEmbeddingService({
        model: 'Xenova/all-MiniLM-L6-v2',
        dimensions: 384,
      });
      await transformers.initialize();
      this.embedder = transformers;
      this.status('init', 'Embedding: TransformersEmbeddingService (384-dim)');
    } catch (err) {
      this.status('init', `Embedding: failed to initialize — ${err instanceof Error ? err.message : String(err)}`);
    }

    // Semantic router (best-effort)
    if (this.embedder) {
      try {
        const routerMod = await afImport('routing/SemanticRouter.js') as typeof import('agentic-flow/dist/routing/SemanticRouter.js');
        this.semanticRouter = new routerMod.SemanticRouter(createRouterEmbedder(this.embedder) as any);
        await this.semanticRouter.registerAgents(CFA_INTENTS);
        this.semanticRouter.buildIndex();
        this.status('init', `SemanticRouter: ${CFA_INTENTS.length} intents indexed`);
      } catch {
        this.semanticRouter = null;
        this.status('init', 'SemanticRouter: unavailable — will use chief analyst fallback');
      }
    }

    // Attention coordinator (best-effort)
    if (this.embedder) {
      try {
        const attMod = await afImport('core/attention-fallbacks.js') as typeof import('agentic-flow/dist/core/attention-fallbacks.js');
        const coordMod = await afImport('coordination/attention-coordinator.js') as typeof import('agentic-flow/dist/coordination/attention-coordinator.js');
        const attention = attMod.createAttention(this.config.attentionMechanism, {
          hiddenDim: 384,
          numHeads: 8,
        });
        this.coordinator = coordMod.createAttentionCoordinator(attention);
        this.status('init', `AttentionCoordinator: ${this.config.attentionMechanism} (384-dim, 8 heads)`);
      } catch {
        this.coordinator = null;
        this.status('init', 'AttentionCoordinator: unavailable — will use equal weights');
      }
    }

    // Learning & memory modules
    // CFA_MEMORY_BACKEND: postgres | sqlite (default) | local
    const memoryBackend = process.env.CFA_MEMORY_BACKEND ?? 'sqlite';

    if (memoryBackend === 'postgres') {
      // Postgres-backed (ruvector-postgres with pgvector search)
      try {
        const { PgReasoningBank } = await import('../learning/pg-reasoning-bank.js');
        const { PgFinancialMemory } = await import('../memory/pg-financial-memory.js');
        const { healthCheck, runMigrations } = await import('../db/pg-client.js');

        const healthy = await healthCheck();
        if (!healthy) throw new Error('Postgres health check failed');

        const migrations = await runMigrations();
        if (migrations.length > 0) {
          this.status('init', `Postgres: ran ${migrations.length} migration(s)`);
        }

        this.reasoningBank = new PgReasoningBank();
        this.financialMemory = new PgFinancialMemory();
        this.status('init', `ReasoningBank: PgReasoningBank (${process.env.PG_HOST ?? 'localhost'}:${process.env.PG_PORT ?? '5433'})`);
        this.status('init', `FinancialMemory: PgFinancialMemory`);
      } catch (err) {
        this.status('init', `Postgres init failed: ${err instanceof Error ? err.message : String(err)} — falling back to sqlite`);
        // Fall through to sqlite/agentdb below
        await this.initSqliteMemory();
      }
    } else if (memoryBackend === 'local') {
      // Pure in-memory (no persistence)
      try {
        const { LocalReasoningBank } = await import('../learning/reasoning-bank.js');
        const { LocalFinancialMemory } = await import('../memory/financial-memory.js');
        this.reasoningBank = new LocalReasoningBank();
        this.financialMemory = new LocalFinancialMemory();
        this.status('init', 'ReasoningBank: LocalReasoningBank (in-memory)');
        this.status('init', 'FinancialMemory: LocalFinancialMemory (in-memory)');
      } catch {
        this.status('init', 'Memory: local backends unavailable');
      }
    } else {
      // Default: sqlite via agentic-flow's ReasoningBank
      await this.initSqliteMemory();
    }

    this.initialized = true;
  }

  private async initSqliteMemory(): Promise<void> {
    try {
      const { SonaReasoningBank } = await import('../learning/reasoning-bank.js');
      this.reasoningBank = new SonaReasoningBank();
      this.status('init', 'ReasoningBank: SonaReasoningBank (sqlite)');
    } catch {
      try {
        const { LocalReasoningBank } = await import('../learning/reasoning-bank.js');
        this.reasoningBank = new LocalReasoningBank();
        this.status('init', 'ReasoningBank: LocalReasoningBank fallback');
      } catch {
        this.status('init', 'ReasoningBank: unavailable');
      }
    }

    try {
      const { AgentDbFinancialMemory } = await import('../memory/financial-memory.js');
      this.financialMemory = new AgentDbFinancialMemory();
      this.status('init', 'FinancialMemory: AgentDbFinancialMemory (sqlite)');
    } catch {
      try {
        const { LocalFinancialMemory } = await import('../memory/financial-memory.js');
        this.financialMemory = new LocalFinancialMemory();
        this.status('init', 'FinancialMemory: LocalFinancialMemory fallback');
      } catch {
        this.status('init', 'FinancialMemory: unavailable');
      }
    }
  }

  // ── Execute: 6-stage pipeline ─────────────────────────────────────

  async execute(
    query: string,
    onStream?: (chunk: string) => void,
  ): Promise<PipelineResult> {
    await this.init();

    const requestId = randomUUID();
    const totalStart = Date.now();
    const timings = { routingMs: 0, memoryMs: 0, agentsMs: 0, coordinationMs: 0, synthesisMs: 0, totalMs: 0 };

    // ── Stage 1: Task Router ──────────────────────────────────────
    this.status('routing', 'Analyzing query intent...');
    const routingStart = Date.now();

    let routedAgentTypes: string[];
    let multiIntent: MultiIntentResult | null = null;

    if (this.semanticRouter) {
      try {
        multiIntent = await this.semanticRouter.detectMultiIntent(query, this.config.confidenceThreshold);

        if (multiIntent.requiresMultiAgent && multiIntent.intents.length > 1) {
          routedAgentTypes = multiIntent.intents
            .slice(0, this.config.maxAgents)
            .map(i => i.agentType);
          this.status('routing', `Multi-intent: ${routedAgentTypes.length} agents [${routedAgentTypes.join(', ')}]`);
        } else if (multiIntent.intents.length > 0) {
          // Single intent — skip to synthesis with one agent
          routedAgentTypes = [multiIntent.intents[0].agentType];
          this.status('routing', `Single intent: ${routedAgentTypes[0]} (confidence: ${multiIntent.intents[0].confidence.toFixed(2)})`);
        } else {
          // detectMultiIntent found nothing above threshold — fall back to route()
          const singleRoute = await this.semanticRouter!.route(query);
          routedAgentTypes = [singleRoute.primaryAgent];
          this.status('routing', `Routed: ${singleRoute.primaryAgent} (confidence: ${singleRoute.confidence.toFixed(2)})`);
        }
      } catch {
        routedAgentTypes = [DEFAULT_AGENT];
        this.status('routing', 'Router error — defaulting to chief analyst');
      }
    } else {
      routedAgentTypes = [DEFAULT_AGENT];
      this.status('routing', 'No router — using chief analyst');
    }

    timings.routingMs = Date.now() - routingStart;

    // ── Stage 2: Vector Search (prior patterns) ───────────────────
    this.status('memory', 'Searching prior patterns and analyses...');
    const memoryStart = Date.now();

    let priorPatterns = '';
    let priorAnalyses = '';

    const primaryTaskType = inferTaskType(routedAgentTypes[0]);

    const [patternsResult, memoryResult] = await Promise.all([
      this.reasoningBank
        ? this.reasoningBank.searchPatterns(primaryTaskType, 5).catch(() => [])
        : Promise.resolve([]),
      this.financialMemory
        ? this.financialMemory.search(query, 3).catch(() => ({ entries: [] }))
        : Promise.resolve({ entries: [] }),
    ]);

    if (patternsResult.length > 0) {
      priorPatterns = patternsResult
        .map(p => `- ${p.taskType}: ${p.toolSequence.join(' → ')} (reward: ${p.rewardScore.toFixed(2)})`)
        .join('\n');
      this.status('memory', `Found ${patternsResult.length} prior patterns`);
    }

    if (memoryResult.entries.length > 0) {
      priorAnalyses = memoryResult.entries
        .map(e => `- [score ${e.similarityScore.toFixed(2)}] ${e.entry.content.slice(0, 200)}...`)
        .join('\n');
      this.status('memory', `Found ${memoryResult.entries.length} prior analyses`);
    }

    timings.memoryMs = Date.now() - memoryStart;

    // ── Stage 3: Spawn Agents ─────────────────────────────────────
    this.status('agents', `Spawning ${routedAgentTypes.length} agent(s)...`);
    const agentsStart = Date.now();

    // Build augmented prompt with context
    const contextBlock = [
      priorPatterns ? `## Prior Successful Tool Sequences\n${priorPatterns}` : '',
      priorAnalyses ? `## Prior Related Analyses\n${priorAnalyses}` : '',
    ].filter(Boolean).join('\n\n');

    const augmentedQuery = contextBlock
      ? `${query}\n\n---\n\n# Context from Prior Analyses\n\n${contextBlock}`
      : query;

    type AgentRunResult = { agentName: string; agentType: string; output: string };

    const agentPromises = routedAgentTypes.map(async (agentType): Promise<AgentRunResult | null> => {
      const agentName = agentNameFromType(agentType);

      try {
        const agentDef = this.getAgent(agentName, cfaAgentsDir) ?? this.getAgent(agentName);
        if (!agentDef) {
          this.status('agents', `Agent ${agentName} not found — skipping`);
          return null;
        }

        const agent = injectSkills(agentDef);

        // No streaming for individual agents — only synthesis streams
        const result = await Promise.race([
          this.claudeAgent(agent, augmentedQuery),
          new Promise<never>((_, reject) =>
            setTimeout(() => reject(new Error(`Agent ${agentName} timed out after ${AGENT_TIMEOUT_MS / 1000}s`)), AGENT_TIMEOUT_MS),
          ),
        ]);

        this.status('agents', `${agentName} complete (${result.output.length} chars)`);
        return { agentName, agentType, output: result.output };
      } catch (err) {
        this.status('agents', `${agentName} failed: ${err instanceof Error ? err.message : String(err)}`);
        return null;
      }
    });

    const rawResults = await Promise.all(agentPromises);
    const agentResults = rawResults.filter((r): r is AgentRunResult => r !== null);

    if (agentResults.length === 0) {
      throw new PipelineError('All agents failed — cannot produce analysis', 'agents');
    }

    this.status('agents', `${agentResults.length}/${routedAgentTypes.length} agents returned results`);
    timings.agentsMs = Date.now() - agentsStart;

    // ── Stage 4: Coordination Layer ───────────────────────────────
    this.status('coordination', 'Running attention-based coordination...');
    const coordStart = Date.now();

    let coordResult: CoordinationResult | null = null;
    let attentionWeights: number[] = agentResults.map(() => 1 / agentResults.length);

    if (this.coordinator && this.embedder && agentResults.length > 1) {
      try {
        // Embed each agent's output
        const embedResults = await Promise.all(
          agentResults.map(r => this.embedder!.embed(r.output.slice(0, 2000))),
        );

        const agentOutputs: AgentOutput[] = agentResults.map((r, i) => ({
          agentId: r.agentName,
          agentType: r.agentType,
          embedding: new Float32Array(embedResults[i].embedding),
          value: new Float32Array(embedResults[i].embedding),
          confidence: multiIntent?.intents.find(intent => intent.agentType === r.agentType)?.confidence ?? 0.7,
        }));

        coordResult = await this.coordinator.topologyAwareCoordination(
          agentOutputs,
          this.config.topology,
        );

        attentionWeights = coordResult.attentionWeights.slice(0, agentResults.length);

        // Normalize weights
        const weightSum = attentionWeights.reduce((s, w) => s + w, 0);
        if (weightSum > 0) {
          attentionWeights = attentionWeights.map(w => w / weightSum);
        }

        this.status('coordination', `Topology: ${this.config.topology} | Top agents: ${coordResult.topAgents.join(', ')}`);
      } catch (err) {
        this.status('coordination', `Coordination failed: ${err instanceof Error ? err.message : String(err)} — using equal weights`);
        attentionWeights = agentResults.map(() => 1 / agentResults.length);
      }
    } else if (agentResults.length === 1) {
      attentionWeights = [1.0];
      this.status('coordination', 'Single agent — skipping coordination');
    }

    // Sort by attention weight (highest first)
    const indexed = agentResults.map((r, i) => ({ ...r, attentionWeight: attentionWeights[i] }));
    indexed.sort((a, b) => b.attentionWeight - a.attentionWeight);

    timings.coordinationMs = Date.now() - coordStart;

    // ── Stage 5: Result Synthesis ─────────────────────────────────
    this.status('synthesis', 'Synthesizing final analysis...');
    const synthStart = Date.now();

    let synthesis: string;

    if (indexed.length === 1) {
      // Single agent — stream directly
      synthesis = indexed[0].output;
      if (onStream) onStream(synthesis);
    } else {
      // Multi-agent — use chief analyst to synthesize
      try {
        const chiefDef = this.getAgent(DEFAULT_AGENT, cfaAgentsDir) ?? this.getAgent(DEFAULT_AGENT);
        if (!chiefDef) throw new Error('Chief analyst agent not found');

        // Inject all 4 skill domains
        const chiefAgent = injectSkills({
          ...chiefDef,
          name: DEFAULT_AGENT,
        });

        const synthesisPrompt = this.buildSynthesisPrompt(query, indexed, coordResult);

        const result = await this.claudeAgent(chiefAgent, synthesisPrompt, onStream);
        synthesis = result.output;
      } catch (err) {
        // Fallback: concatenate raw outputs
        this.status('synthesis', `Synthesis agent failed: ${err instanceof Error ? err.message : String(err)} — concatenating raw outputs`);
        synthesis = indexed
          .map(r => `## ${r.agentName} (weight: ${r.attentionWeight.toFixed(2)})\n\n${r.output}`)
          .join('\n\n---\n\n');
        if (onStream) onStream(synthesis);
      }
    }

    timings.synthesisMs = Date.now() - synthStart;
    timings.totalMs = Date.now() - totalStart;

    // ── Stage 6: Learning ─────────────────────────────────────────
    if (this.config.enableLearning) {
      this.recordLearning(requestId, query, indexed, synthesis).catch(() => {
        // Silently ignore learning failures
      });
    }

    const pipelineResult: PipelineResult = {
      requestId,
      synthesis,
      agentResults: indexed,
      routedAgents: routedAgentTypes,
      topology: this.config.topology,
      coordination: coordResult ? {
        mechanism: coordResult.mechanism,
        executionTimeMs: coordResult.executionTimeMs,
        topAgents: coordResult.topAgents,
        attentionWeights,
      } : undefined,
      timings,
    };

    return pipelineResult;
  }

  // ── Synthesis prompt builder ───────────────────────────────────

  private buildSynthesisPrompt(
    query: string,
    rankedResults: AgentResult[],
    coordResult: CoordinationResult | null,
  ): string {
    const agentSections = rankedResults
      .map((r, i) => {
        const rank = i + 1;
        return `### Agent ${rank}: ${r.agentName} (attention weight: ${r.attentionWeight.toFixed(3)})\n\n${r.output}`;
      })
      .join('\n\n---\n\n');

    const coordMeta = coordResult
      ? `\n\n## Coordination Metadata\n- Mechanism: ${coordResult.mechanism}\n- Topology: ${this.config.topology}\n- Top agents: ${coordResult.topAgents.join(', ')}\n- Execution: ${coordResult.executionTimeMs}ms`
      : '';

    return `You are synthesizing results from ${rankedResults.length} specialist financial analysts into a single, coherent institutional-grade response.

## Original User Query

${query}

## Agent Results (ranked by attention weight)

${agentSections}
${coordMeta}

## Synthesis Instructions

1. Integrate findings from all agents, weighting more heavily those with higher attention scores.
2. Resolve any contradictions by noting the disagreement and the reasoning on each side.
3. Present a unified analysis with clear sections, numerical precision, and base/bull/bear scenarios where applicable.
4. Every number must trace to a specific agent's tool output — do not generate new calculations.
5. Include a brief "Methodology" section listing which agents contributed and their key tools.
6. End with "Key Risks" and "Confidence Assessment".`;
  }

  // ── Learning recorder ──────────────────────────────────────────

  private async recordLearning(
    requestId: string,
    query: string,
    results: AgentResult[],
    synthesis: string,
  ): Promise<void> {
    // Record reasoning trace
    if (this.reasoningBank) {
      try {
        await this.reasoningBank.recordTrace({
          traceId: randomUUID(),
          agentType: 'cfa-pipeline',
          requestId,
          steps: [
            { phase: 'observe', content: query, timestamp: new Date() },
            ...results.map(r => ({
              phase: 'act' as const,
              content: `${r.agentName}: ${r.output.slice(0, 200)}`,
              toolCalls: [r.agentType],
              timestamp: new Date(),
            })),
            { phase: 'reflect', content: synthesis.slice(0, 500), timestamp: new Date() },
          ],
          outcome: 'success',
          createdAt: new Date(),
        });
      } catch { /* best-effort */ }
    }

    // Store synthesis in financial memory
    if (this.financialMemory) {
      try {
        await this.financialMemory.store(synthesis.slice(0, 5000), {
          sourceType: 'analysis',
          analysisType: 'pipeline-synthesis',
          tags: results.map(r => r.agentType),
        });
      } catch { /* best-effort */ }
    }
  }

  // ── Status emitter ─────────────────────────────────────────────

  private status(stage: string, message: string): void {
    this.config.onStatus?.(stage, message);
  }
}
