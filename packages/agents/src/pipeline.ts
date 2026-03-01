// CFA Multi-Agent Pipeline — 6-stage orchestration

import { randomUUID } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';
import { createRequire } from 'node:module';
import type { AgentOutput, CoordinationResult } from 'agentic-flow/dist/coordination/attention-coordinator.js';
import type { MultiIntentResult } from 'agentic-flow/dist/routing/SemanticRouter.js';
import type { ReasoningBank } from '../learning/reasoning-bank.js';
import type { FinancialMemory } from '../memory/financial-memory.js';

export type { Topology, PipelineConfig, AgentResult, PipelineResult } from './pipeline-config.js';
export { PipelineError, injectSkills } from './pipeline-config.js';

import {
  type PipelineConfig, type AgentResult, type PipelineResult,
  PipelineError, DEFAULT_CONFIG, AGENT_TIMEOUT_MS, AGENT_MAX_TURNS,
  DEFAULT_AGENT, CFA_INTENTS, agentNameFromType, inferTaskType,
  createRouterEmbedder, injectSkills,
} from './pipeline-config.js';
import { buildAgentPreamble, resolveTickerFromQuery } from './pipeline-fmp.js';
import { runAgentWithAbort } from './pipeline-agents.js';

const __pipelineDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__pipelineDir, '..', '..', '..');
const cfaAgentsDir = join(repoRoot, '.claude', 'agents', 'cfa');
const skillsDir = join(repoRoot, '.claude', 'skills');

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

  private async init(): Promise<void> {
    if (this.initialized) return;

    const _require = createRequire(import.meta.url);
    const _afDir = dirname(_require.resolve('agentic-flow/package.json'));
    const afImport = (subpath: string) =>
      import(pathToFileURL(join(_afDir, 'dist', ...subpath.split('/'))).href);

    const [agentMod, loaderMod] = await Promise.all([
      afImport('agents/claudeAgent.js') as Promise<typeof import('agentic-flow/dist/agents/claudeAgent.js')>,
      afImport('utils/agentLoader.js') as Promise<typeof import('agentic-flow/dist/utils/agentLoader.js')>,
    ]);
    this.claudeAgent = agentMod.claudeAgent;
    this.getAgent = loaderMod.getAgent;

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

    const memoryBackend = process.env.CFA_MEMORY_BACKEND ?? 'sqlite';

    if (memoryBackend === 'postgres') {
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
        await this.initSqliteMemory();
      }
    } else if (memoryBackend === 'local') {
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

  async execute(
    query: string,
    onStream?: (chunk: string) => void,
  ): Promise<PipelineResult> {
    await this.init();

    const requestId = randomUUID();
    const totalStart = Date.now();
    const timings = { routingMs: 0, memoryMs: 0, agentsMs: 0, coordinationMs: 0, synthesisMs: 0, totalMs: 0 };

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
          routedAgentTypes = [multiIntent.intents[0].agentType];
          this.status('routing', `Single intent: ${routedAgentTypes[0]} (confidence: ${multiIntent.intents[0].confidence.toFixed(2)})`);
        } else {
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

    const resolvedTicker = await resolveTickerFromQuery(query, __pipelineDir);
    if (resolvedTicker) {
      this.status('routing', `Ticker resolved: ${resolvedTicker}`);
    }

    this.status('agents', `Spawning ${routedAgentTypes.length} agent(s)...`);
    const agentsStart = Date.now();

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

        const agent = injectSkills(agentDef, skillsDir);
        const agentPrompt = buildAgentPreamble(agentType, query, __pipelineDir, resolvedTicker) + augmentedQuery;

        const result = await runAgentWithAbort(agent, agentPrompt, {
          timeoutMs: AGENT_TIMEOUT_MS,
          maxTurns: AGENT_MAX_TURNS,
          onToolCall: (name, count) => {
            const ts = new Date().toISOString().split('T')[1].split('.')[0];
            process.stderr.write(`\n[${ts}] 🔍 Tool call #${count}: ${name}\n`);
          },
        });

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

    this.status('coordination', 'Running attention-based coordination...');
    const coordStart = Date.now();

    let coordResult: CoordinationResult | null = null;
    let attentionWeights: number[] = agentResults.map(() => 1 / agentResults.length);

    if (this.coordinator && this.embedder && agentResults.length > 1) {
      try {
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

    const indexed = agentResults.map((r, i) => ({ ...r, attentionWeight: attentionWeights[i] }));
    indexed.sort((a, b) => b.attentionWeight - a.attentionWeight);

    timings.coordinationMs = Date.now() - coordStart;

    this.status('synthesis', 'Synthesizing final analysis...');
    const synthStart = Date.now();

    let synthesis: string;

    if (indexed.length === 1) {
      synthesis = indexed[0].output;
      if (onStream) onStream(synthesis);
    } else {
      try {
        const chiefDef = this.getAgent(DEFAULT_AGENT, cfaAgentsDir) ?? this.getAgent(DEFAULT_AGENT);
        if (!chiefDef) throw new Error('Chief analyst agent not found');

        const chiefAgent = injectSkills({
          ...chiefDef,
          name: DEFAULT_AGENT,
        }, skillsDir);

        const synthesisPrompt = this.buildSynthesisPrompt(query, indexed, coordResult);

        const result = await runAgentWithAbort(chiefAgent, synthesisPrompt, {
          timeoutMs: AGENT_TIMEOUT_MS,
          maxTurns: AGENT_MAX_TURNS,
        });
        synthesis = result.output;
        if (onStream) onStream(synthesis);
      } catch (err) {
        this.status('synthesis', `Synthesis agent failed: ${err instanceof Error ? err.message : String(err)} — concatenating raw outputs`);
        synthesis = indexed
          .map(r => `## ${r.agentName} (weight: ${r.attentionWeight.toFixed(2)})\n\n${r.output}`)
          .join('\n\n---\n\n');
        if (onStream) onStream(synthesis);
      }
    }

    timings.synthesisMs = Date.now() - synthStart;
    timings.totalMs = Date.now() - totalStart;

    if (this.config.enableLearning) {
      this.recordLearning(requestId, query, indexed, synthesis).catch(() => {});
    }

    return {
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
  }

  private buildSynthesisPrompt(
    query: string,
    rankedResults: AgentResult[],
    coordResult: CoordinationResult | null,
  ): string {
    const agentSections = rankedResults
      .map((r, i) => `### Agent ${i + 1}: ${r.agentName} (attention weight: ${r.attentionWeight.toFixed(3)})\n\n${r.output}`)
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

  private async recordLearning(
    requestId: string,
    query: string,
    results: AgentResult[],
    synthesis: string,
  ): Promise<void> {
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

  private status(stage: string, message: string): void {
    this.config.onStatus?.(stage, message);
  }
}
