// Base analyst agent with Dexter-style iterative reasoning loop
// All specialist agents extend this class

import { randomUUID } from 'node:crypto';
import type {
  AgentType, AgentCapability, ToolInvocation, Finding,
  Citation, AnalysisResult,
} from '../types/agents.js';
import type { DomainEvent, EventBus } from '../types/events.js';
import { TOOL_MAPPINGS, AGENT_DESCRIPTIONS } from '../config/tool-mappings.js';
import { parseFinancialData, type ExtractedMetrics } from '../utils/financial-parser.js';
import { enrichMetrics } from '../utils/fmp-data-fetcher.js';
import type { InsightBus } from '../collaboration/insight-bus.js';

export interface AnalystContext {
  assignmentId: string;
  requestId: string;
  task: string;
  /** Company name resolved by orchestrator — bypasses regex extraction */
  company?: string;
  priorContext?: string;       // relevant memory from prior analyses
  eventBus: EventBus;
  callTool: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  callFmpTool?: (toolName: string, params: Record<string, unknown>) => Promise<unknown>;
  /** ADR-006: Cross-specialist collaboration bus */
  insightBus?: InsightBus;
}

export interface ReasoningState {
  observations: string[];
  thoughts: string[];
  toolResults: ToolInvocation[];
  reflections: string[];
  iteration: number;
  maxIterations: number;
  shouldContinue: boolean;
  /** Financial metrics extracted from the task text */
  metrics: ExtractedMetrics;
}

export abstract class BaseAnalyst {
  readonly agentId: string;
  readonly agentType: AgentType;
  readonly capability: AgentCapability;

  constructor(agentType: AgentType) {
    this.agentId = randomUUID();
    this.agentType = agentType;
    this.capability = {
      agentType,
      toolDomains: TOOL_MAPPINGS[agentType] ?? [],
      analysisTypes: [],
      description: AGENT_DESCRIPTIONS[agentType] ?? '',
    };
  }

  // The Dexter-style iterative reasoning loop
  async execute(ctx: AnalystContext): Promise<AnalysisResult> {
    const textMetrics = parseFinancialData(ctx.task);

    // Orchestrator-resolved company takes priority over regex extraction
    if (ctx.company) {
      textMetrics._company = ctx.company;
    }

    const state: ReasoningState = {
      observations: [],
      thoughts: [],
      toolResults: [],
      reflections: [],
      iteration: 0,
      maxIterations: 5,
      shouldContinue: true,
      metrics: textMetrics,
    };

    // Enrich with live FMP data if available
    if (ctx.callFmpTool && textMetrics._company) {
      try {
        state.metrics = await enrichMetrics(textMetrics, ctx.callFmpTool);
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        state.observations.push(`[FMP enrichment failed: ${errorMsg}] Continuing with text-only metrics`);
        ctx.eventBus.emit({
          eventId: randomUUID(),
          type: 'ToolFailed',
          timestamp: new Date(),
          sourceContext: 'SpecialistAnalysts',
          payload: { invocationId: 'fmp-enrichment', errorType: `FMP enrichment: ${errorMsg}` },
        });
      }
    }

    // Phase 1: Observe
    state.observations.push(
      `Assignment: ${ctx.task}`,
      ...(ctx.priorContext ? [`Prior context: ${ctx.priorContext}`] : []),
    );

    // ADR-006: Subscribe to peer insights if InsightBus is available
    if (ctx.insightBus) {
      ctx.insightBus.subscribe(this.agentId, (insight) => {
        state.observations.push(
          `[Peer insight from ${insight.sourceAgent}] ${insight.content}`,
        );
      });
    }

    while (state.shouldContinue && state.iteration < state.maxIterations) {
      state.iteration++;

      // ADR-006: Inject peer insights into observations before thinking
      if (ctx.insightBus && state.iteration > 1) {
        const peerContext = ctx.insightBus.formatPeerContext(this.agentId, 0.6);
        if (peerContext) {
          state.observations.push(`Peer findings:\n${peerContext}`);
        }
      }

      // Phase 2: Think — plan which tools to call
      const toolPlan = await this.think(ctx, state);

      // Phase 3: Act — execute tool calls
      for (const { toolName, params } of toolPlan) {
        if (!this.canUseTool(toolName)) continue;

        const invocation: ToolInvocation = {
          invocationId: randomUUID(),
          agentId: this.agentId,
          toolName,
          params,
          timestamp: new Date(),
        };

        ctx.eventBus.emit({
          eventId: randomUUID(),
          type: 'ToolCalled',
          timestamp: new Date(),
          sourceContext: 'SpecialistAnalysts',
          payload: { agentId: this.agentId, toolName, params, invocationId: invocation.invocationId },
        });

        try {
          const start = Date.now();
          invocation.result = await ctx.callTool(toolName, params);
          invocation.duration = Date.now() - start;

          ctx.eventBus.emit({
            eventId: randomUUID(),
            type: 'ToolSucceeded',
            timestamp: new Date(),
            sourceContext: 'SpecialistAnalysts',
            payload: { invocationId: invocation.invocationId, duration: invocation.duration },
          });
        } catch (err) {
          invocation.error = err instanceof Error ? err.message : String(err);
          ctx.eventBus.emit({
            eventId: randomUUID(),
            type: 'ToolFailed',
            timestamp: new Date(),
            sourceContext: 'SpecialistAnalysts',
            payload: { invocationId: invocation.invocationId, errorType: invocation.error },
          });
        }

        state.toolResults.push(invocation);
      }

      // Phase 4: Reflect — evaluate results, decide to iterate or finalize
      const reflection = await this.reflect(ctx, state);
      state.reflections.push(reflection.summary);
      state.shouldContinue = reflection.shouldIterate;

      // ADR-006: Broadcast top findings to peers after each iteration
      if (ctx.insightBus) {
        this.broadcastFindings(ctx, state);
      }
    }

    // ADR-006: Unsubscribe from insight bus
    if (ctx.insightBus) {
      ctx.insightBus.unsubscribe(this.agentId);
    }

    // Phase 5: Report — produce structured findings
    const findings = await this.synthesize(ctx, state);
    const confidence = this.calculateConfidence(state);

    const result: AnalysisResult = {
      resultId: randomUUID(),
      agentId: this.agentId,
      agentType: this.agentType,
      assignmentId: ctx.assignmentId,
      findings,
      summary: this.buildSummary(findings, state),
      confidence,
      toolInvocations: state.toolResults,
      completedAt: new Date(),
    };

    ctx.eventBus.emit({
      eventId: randomUUID(),
      type: 'AnalysisCompleted',
      timestamp: new Date(),
      sourceContext: 'SpecialistAnalysts',
      payload: { agentId: this.agentId, assignmentId: ctx.assignmentId, resultId: result.resultId, confidence },
    });

    return result;
  }

  // Subclasses implement domain-specific reasoning
  protected abstract think(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Array<{ toolName: string; params: Record<string, unknown> }>>;

  protected abstract reflect(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<{ summary: string; shouldIterate: boolean }>;

  protected abstract synthesize(
    ctx: AnalystContext,
    state: ReasoningState,
  ): Promise<Finding[]>;

  protected canUseTool(toolName: string): boolean {
    // Check if the tool's module is in this agent's allowed domains
    return this.capability.toolDomains.some(domain =>
      toolName.startsWith(domain) || toolName.includes(domain),
    );
  }

  /**
   * ADR-006: Broadcast key findings from the current iteration to peers.
   * Extracts the most significant tool results and broadcasts them.
   */
  private broadcastFindings(ctx: AnalystContext, state: ReasoningState): void {
    if (!ctx.insightBus) return;

    // Broadcast successful tool results from the latest iteration
    const recentResults = state.toolResults
      .filter(t => !t.error && t.result)
      .slice(-3); // Last 3 results from this iteration

    for (const tool of recentResults) {
      const resultStr = typeof tool.result === 'string'
        ? tool.result.slice(0, 200)
        : JSON.stringify(tool.result).slice(0, 200);

      ctx.insightBus.broadcast({
        sourceAgent: this.agentType,
        sourceAgentId: this.agentId,
        insightType: 'finding',
        content: `${tool.toolName}: ${resultStr}`,
        data: { toolName: tool.toolName, result: tool.result },
        confidence: this.calculateConfidence(state),
      });
    }

    // Broadcast reflections as higher-level insights
    if (state.reflections.length > 0) {
      const latestReflection = state.reflections[state.reflections.length - 1];
      ctx.insightBus.broadcast({
        sourceAgent: this.agentType,
        sourceAgentId: this.agentId,
        insightType: 'finding',
        content: latestReflection,
        data: { iteration: state.iteration },
        confidence: this.calculateConfidence(state),
      });
    }
  }

  private calculateConfidence(state: ReasoningState): number {
    const successfulTools = state.toolResults.filter(t => !t.error).length;
    const totalTools = state.toolResults.length;
    if (totalTools === 0) return 0;

    // Base confidence from tool success ratio
    let confidence = successfulTools / totalTools;

    // Boost for multiple iterations (more thorough analysis)
    if (state.iteration > 1) confidence = Math.min(1, confidence * 1.05);

    // Boost for FMP-enriched data (real market data vs text-only)
    if (state.metrics._dataSource === 'fmp-enriched') {
      confidence = Math.min(1, confidence * 1.1);
    } else if (state.metrics._dataSource === 'text-only') {
      confidence *= 0.85; // Penalize text-only analyses slightly
    }

    // Penalize if very few tools were called (insufficient analysis)
    if (successfulTools < 2) {
      confidence *= 0.8;
    }

    return Math.round(confidence * 100) / 100; // Round to 2 decimal places
  }

  private buildSummary(findings: Finding[], state: ReasoningState): string {
    if (findings.length === 0) return 'No findings produced.';

    const dataSource = state.metrics._dataSource === 'fmp-enriched'
      ? 'live market data' : 'text-extracted metrics';
    const company = state.metrics._company ?? 'the subject';
    const successCount = state.toolResults.filter(t => !t.error).length;
    const totalCount = state.toolResults.length;

    const header = `Analysis of ${company} using ${dataSource} (${successCount}/${totalCount} tools succeeded across ${state.iteration} iteration(s)):`;

    // Group findings by methodology and extract key statements
    const findingLines = findings
      .filter(f => f.confidence > 0)
      .map(f => `- [${f.methodology}] ${f.statement.slice(0, 200)}`)
      .join('\n');

    return `${header}\n${findingLines}`;
  }
}
